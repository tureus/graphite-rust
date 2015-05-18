// use std::io::Error;
use std::fs::File;
use std::io::{SeekFrom, Seek, Read, Write};
use std::fs::OpenOptions;
use std::fmt;
use num::iter::range_step_inclusive;
use std::cell::RefCell;

use super::header::{ Header, read_header };
use super::write_op::WriteOp;
use super::archive_info::ArchiveInfo;
use super::metadata::{AggregationType};

use whisper::point;

pub struct WhisperFile<'a> {
    pub path: &'a str,
    pub handle: RefCell<File>,
    pub header: Header
}

impl<'a> fmt::Debug for WhisperFile<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "path:  {:?}, header: {:?}", self.path, self.header)
    }
}

// TODO: Change error value to generic Error
pub fn open(path: &str) -> Result<WhisperFile, &'static str> {
    let file_handle = OpenOptions::new().read(true).write(true).create(false).open(path);

    match file_handle {
        Ok(f) => {
            let header = try!(read_header(&f));
            let whisper_file = WhisperFile { path: path, header: header, handle: RefCell::new(f) };
            Ok( whisper_file )
        },
        Err(_) => {
            Err("generic file error")
        }
    }
}

impl<'a> WhisperFile<'a> {
    // TODO: Result<usize> return how many write ops were done
    pub fn write(&mut self, current_time: u64, point: point::Point) {
        let pair = {
            self.split_archives(current_time, point.timestamp)
        };

        match pair {
            Some( (high_precision_archive, rest) ) => {
                let base_point = {
                    self.read_point(high_precision_archive.offset)
                };
                let base_timestamp = base_point.timestamp;
                self.calculate_write_ops(
                    (high_precision_archive, rest),
                    point,
                    base_timestamp
                );
            },
            None => {
                panic!("no archives satisfy current time")
            }
        }
    }

    pub fn perform_write_op(&self, write_op: &WriteOp) {
        debug!("writing to pos {:?} of {}", write_op.seek, self.path);
        let mut handle = self.handle.borrow_mut();
        handle.seek(write_op.seek).unwrap();
        handle.write_all(&(write_op.bytes)).unwrap();
    }

    fn read_point(&self, offset: u64) -> point::Point {
        let mut file = self.handle.borrow_mut();
        let seek = file.seek(SeekFrom::Start(offset));

        match seek {
            Ok(_) => {
                let mut points_buf : [u8; 12] = [0; 12];
                let mut buf_ref : &mut [u8] = &mut points_buf;
                let read = file.read(buf_ref);

                match read {
                    Ok(_) => point::buf_to_point(buf_ref),
                    Err(err) => {
                        panic!("read point {:?}", err)
                    }
                }
            },
            Err(err) => {
                panic!("read_point {:?}", err)
            }
        }
    }

    pub fn calculate_write_ops(&self, (ai,rest): (&ArchiveInfo, Vec<&ArchiveInfo>), point: point::Point, base_timestamp: u64) {
        {
            let write_op = build_write_op( ai, &point, base_timestamp );
            self.perform_write_op(&write_op);
        }

        if rest.len() > 1 {
            // debug!("downsampling first pair");
            self.downsample(ai, rest[0], point.timestamp).map(|write_op| self.perform_write_op(&write_op) );

            // debug!("downsampling all others");
            let high_res_iter = rest[0..rest.len()-1].into_iter();
            let low_res_iter = rest[1..].into_iter();
            let res : Vec<()> = high_res_iter.
                zip(low_res_iter).
                // map(|(h,l)| {
                //     self.downsample(*h, *l, point.timestamp)
                // }).
                take_while(|&pair| {
                    let (h,l) = pair;
                    // println!("h: {:?}, l: {:?}", h, l);
                    // true
                    let op = self.downsample(h, l, point.timestamp);
                    match op {
                        Some(write_op) => {
                            self.perform_write_op(&write_op);
                            true
                        },
                        None => false
                    }
                }).
                map(|_| ()).
                collect();
            debug!("res: {:?}", res);
        }
    }

    // The most expensive IO functionality
    // Reads many samples from high-res archive and
    // aggregates to lower-res archive. Schemas could do well to avoid
    // aggregation unless disk space is truly at a premium.
    //
    // A cache for each archive would do well here. `memmap` would be awesomesauce.
    fn downsample(&self, h_res_archive: &ArchiveInfo, l_res_archive: &ArchiveInfo, base_timestamp: u64) -> Option<WriteOp> {
        assert!(h_res_archive.seconds_per_point < l_res_archive.seconds_per_point);

        let l_interval_start = l_res_archive.interval_ceiling(base_timestamp);
        // debug!("base_timestamp: {}, l_interval_start: {}", base_timestamp, l_interval_start);

        let h_base_timestamp = self.read_point(h_res_archive.offset).timestamp;
        let h_res_start_offset = if h_base_timestamp == 0 {
            h_res_archive.offset
        } else {
            // TODO: this can be negative. Does that change timestamp understanding?
            let timespan  = (l_interval_start as i64 - h_base_timestamp as i64).abs() as u64;
            let points = timespan / h_res_archive.seconds_per_point;
            let bytes = points / point::POINT_SIZE as u64;
            let wrapped_index = bytes % h_res_archive.size_in_bytes;
            h_res_archive.offset + wrapped_index
        };

        // debug!("l_res_archive.seconds_per_point: {}, h_res_archive.seconds_per_point: {}", l_res_archive.seconds_per_point, h_res_archive.seconds_per_point);
        let h_res_points_needed = l_res_archive.seconds_per_point / h_res_archive.seconds_per_point;
        let h_res_bytes_needed = h_res_points_needed * point::POINT_SIZE as u64;

        let h_res_end_offset = {
            let rel_first_offset = h_res_start_offset - h_res_archive.offset;
            let rel_second_offset = (rel_first_offset + h_res_bytes_needed) % h_res_archive.size_in_bytes;
            h_res_archive.offset + rel_second_offset
        };

        // debug!("h_res_points_needed: {}, bytes needed: {}", h_res_points_needed, h_res_bytes_needed);
        let mut h_res_read_buf : Vec<u8> = Vec::with_capacity(h_res_bytes_needed as usize);

        // Subroutine for filling in the buffer
        {
            let mut handle = self.handle.borrow_mut();

            // TODO: must be a better way of zeroing out a buffer to fill in the vector
            for _ in 0..h_res_bytes_needed {
                h_res_read_buf.push(0);
            }


            if h_res_start_offset < h_res_end_offset {
                // No wrap situation
                let seek = SeekFrom::Start(h_res_start_offset);

                let mut read_buf : &mut [u8] = &mut h_res_read_buf[..];
                handle.seek(seek).unwrap();
                handle.read(read_buf).unwrap();
            } else {
                let first_seek = SeekFrom::Start(h_res_start_offset);
                let first_seek_bytes = h_res_end_offset - h_res_start_offset;

                let (first_buf, second_buf) = h_res_read_buf.split_at_mut(first_seek_bytes as usize);

                handle.seek(first_seek).unwrap();
                handle.read(first_buf).unwrap();

                let second_seek = SeekFrom::Start(h_res_end_offset);
                handle.seek(second_seek).unwrap();
                handle.read(second_buf).unwrap();
            }

        }

        let low_res_aggregate = {
            // debug!("chunks expected: {}", h_res_read_buf.len() as f32 / point::POINT_SIZE as f32);
            let points : Vec<point::Point> = h_res_read_buf.chunks(point::POINT_SIZE).map(|chunk| {
                // debug!("chunk: {:?}", chunk);
                point::buf_to_point(chunk)
            }).collect();

            let timestamp_start = l_interval_start;
            let timestamp_stop = l_interval_start + (h_res_points_needed as u64)*h_res_archive.seconds_per_point;
            let step = h_res_archive.seconds_per_point;

            let expected_timestamps = range_step_inclusive(timestamp_start, timestamp_stop, step);
            let valid_points : Vec<Option<&point::Point>> = expected_timestamps.
                zip(points.iter()).
                map(|(ts, p)| {
                    debug!("comparing {} and {:?}", ts, p);
                    if p.timestamp == ts {
                        Some(p)
                    } else {
                        None
                    }
                }).collect();

            debug!("expected timestamps: {:?}", valid_points);
            // self.aggregate_samples(points)
            None
        };

        low_res_aggregate.map(|aggregate| {
            let l_res_base_point = self.read_point(l_res_archive.offset);
            let l_res_point = point::Point{ timestamp: l_interval_start, value: aggregate };
            build_write_op(l_res_archive, &l_res_point, l_res_base_point.timestamp)
        })
    }

    fn aggregate_samples(&self, points: Vec<point::Point>) -> Option<f64>{
        debug!("points: {:?}", points);
        let valid_points : Vec<&point::Point> = points.iter().filter(|p| p.timestamp != 0).map(|p| p).collect();

        let ratio : f32 = valid_points.len() as f32 / points.len() as f32;
        debug!("valid_points: {}, len: {}, ratio {} vs xff {}", valid_points.len(), points.len(), ratio, self.header.metadata.x_files_factor);
        if ratio < self.header.metadata.x_files_factor {
            debug!("not aggregating samples");
            return None;
        }

        // TODO: we only do aggregation right now!
        match self.header.metadata.aggregation_type {
            AggregationType::Average => {
                let sum = points.iter().map(|p| p.value).fold(0.0, |l, r| l + r);
                Some(sum / points.len() as f64)
            },
            _ => { Some(0.0) }
        }
    }

    fn split_archives(&self, current_time: u64, point_timestamp: u64) -> Option<(&ArchiveInfo, Vec<&ArchiveInfo>)>  {
        let mut archive_iter = self.header.archive_infos.iter();
        
        let high_precision_archive_option = archive_iter.find(|ai|
            ai.retention > (current_time - point_timestamp)
        );

        match high_precision_archive_option {
            Some(ai) => {
                let rest_of_archives = archive_iter;
                let low_res_archives : Vec<&ArchiveInfo> = rest_of_archives.collect();

                Some((ai, low_res_archives))
            },
            None => {
                None
            }
        }
    }
}

fn build_write_op(archive_info: &ArchiveInfo, point: &point::Point, base_timestamp: u64) -> WriteOp {
    let mut output_data = [0; 12];
    let interval_ceiling = archive_info.interval_ceiling(point.timestamp);
    // debug!("interval_ceiling: {}", interval_ceiling);

    {
        let point_value = point.value;
        let buf : &mut [u8] = &mut output_data;
        point::fill_buf(buf, interval_ceiling, point_value);
    }

    let seek_info = archive_info.calculate_seek(&point,  base_timestamp);

    return WriteOp {
        seek: seek_info,
        bytes: output_data
    }
}


#[test]
fn has_write_ops(){
    use std::io::SeekFrom;
    use super::metadata::Metadata;

    let path = "test/fixtures/60-1440.wsp";
    let high_res_archive = ArchiveInfo {
        offset: 28,
        seconds_per_point: 60,
        points: 1440,
        retention: 60 * 1440,
        size_in_bytes: 1440*12
    };
    let low_res_archive = ArchiveInfo {
        offset: 56,
        seconds_per_point: 60,
        points: 1440,
        retention: 60 * 1440,
        size_in_bytes: 1440*12
    };
    let whisper_file = WhisperFile{
        path: path,
        handle: RefCell::new(File::open(path).unwrap()),
        header: Header {
            metadata: Metadata {
                aggregation_type: AggregationType::Average,
                max_retention: 86400,
                x_files_factor: 1056964608,
                archive_count: 1
            },
            archive_infos: vec![
                high_res_archive,
                low_res_archive
            ]
        }
    };

    let base_timestamp = 10;
    let split_archives = (&high_res_archive, vec![&low_res_archive]);
    let write_ops = whisper_file.calculate_write_ops(
        split_archives,
        point::Point{value: 0.0, timestamp: 10},
        base_timestamp
    );

    let expected = vec![
        WriteOp{seek: SeekFrom::Start(28), bytes: [0,0,0,0,0,0,0,0,0,0,0,0]},
    ];
    assert_eq!(write_ops.len(), expected.len());
    assert_eq!(write_ops, expected);

    return;
}
