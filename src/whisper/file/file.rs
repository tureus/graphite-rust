// use std::io::Error;
use std::fs::File;
use std::io::{SeekFrom, Seek, Read, Write};
use std::fs::OpenOptions;
use std::fmt;
use num::iter::range_step_inclusive;
use std::cell::RefCell;

use super::header::{ Header, read_header };
use super::write_op::WriteOp;
use super::archive_info::{ ArchiveInfo };
use super::metadata::{Metadata, AggregationType};
use whisper::schema::Schema;

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
    pub fn new(path: &str, schema: Schema, metadata: Metadata) -> WhisperFile {
        let size_on_disk = schema.size_on_disk();
        debug!("size_on_disk: {}", size_on_disk);

        // zero-file to that size
        // write header
        // write metadata

        // let file_handle = OpenOptions::new().read(false).write(true).create(true).open(path);

        panic!("hey!")
    }

    // TODO: Result<usize> return how many write ops were done
    pub fn write(&mut self, current_time: u64, point: point::Point) {

        match self.split(current_time, point.timestamp) {
            Some( (high_precision_archive, rest) ) => {
                let base_point = self.read_point(high_precision_archive.offset);
                let base_timestamp = base_point.timestamp;

                self.write_archives(
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

    fn perform_write_op(&self, write_op: &WriteOp) {
        let mut handle = self.handle.borrow_mut();
        handle.seek(write_op.seek).unwrap();
        handle.write_all(&(write_op.bytes)).unwrap();
    }

    fn read_point(&self, offset: u64) -> point::Point {
        let mut file = self.handle.borrow_mut();
        file.seek(SeekFrom::Start(offset)).unwrap();

        let mut points_buf : [u8; 12] = [0; 12];
        let mut buf_ref : &mut [u8] = &mut points_buf;
        file.read(buf_ref).unwrap();

        point::buf_to_point(buf_ref)
    }

    pub fn write_archives(&self, (ai,rest): (&ArchiveInfo, Vec<&ArchiveInfo>), point: point::Point, base_timestamp: u64) {
        {
            let write_op = build_write_op( ai, &point, base_timestamp );
            self.perform_write_op(&write_op);
        }

        if rest.len() > 1 {
            self.downsample(ai, rest[0], point.timestamp).map(|write_op| self.perform_write_op(&write_op) );

            let high_res_iter = rest[0..rest.len()-1].into_iter();
            let low_res_iter = rest[1..].into_iter();
            let _ : Vec<()> = high_res_iter.
                zip(low_res_iter).
                take_while(|&(h,l)| {
                    match self.downsample(h, l, point.timestamp) {
                        Some(write_op) => {
                            self.perform_write_op(&write_op);
                            true
                        },
                        None => false
                    }
                }).
                map(|_| ()).
                collect();
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
        let h_res_start_offset : u64 = if h_base_timestamp == 0 {
            h_res_archive.offset
        } else {
            // TODO: this can be negative. Does that change timestamp understanding?
            let timespan  = l_interval_start as i64 - h_base_timestamp as i64;
            // debug!("timespan {}", timespan);
            let points = timespan / h_res_archive.seconds_per_point as i64;
            // debug!("points {}", points);
            let bytes = points * point::POINT_SIZE as i64;

            // TODO: OMG, move this craziness somewhere else
            let wrapped_index = {
                let remainder = bytes % h_res_archive.size_in_bytes as i64;
                if remainder < 0 {
                    h_res_archive.size_in_bytes as i64 + remainder
                } else {
                    remainder
                }
            };
            // debug!("offset: {}, wrapped_index: {}, bytes: {}, points: {}", h_res_archive.offset, wrapped_index, bytes, points);
            (h_res_archive.offset as i64 + wrapped_index) as u64
        };

        let h_res_points_needed = l_res_archive.seconds_per_point / h_res_archive.seconds_per_point;
        let h_res_bytes_needed = h_res_points_needed * point::POINT_SIZE as u64;

        let h_res_end_offset = {
            let rel_first_offset = h_res_start_offset - h_res_archive.offset;
            let rel_second_offset = (rel_first_offset + h_res_bytes_needed) % h_res_archive.size_in_bytes;
            h_res_archive.offset + rel_second_offset
        };

        let mut h_res_read_buf : Vec<u8> = Vec::with_capacity(h_res_bytes_needed as usize);

        // Subroutine for filling in the buffer
        {
            let mut handle = self.handle.borrow_mut();

            // TODO: must be a better way of zeroing out a buffer to fill in the vector
            for _ in 0..h_res_bytes_needed {
                h_res_read_buf.push(0);
            }

            // TODO: refactor in to function which
            // returns ((Seek,BytesRead),Option<(Seek,BytesRead)>)
            // so this code can be refactored and unit tested...
            if h_res_start_offset < h_res_end_offset {
                // No wrap situation
                let seek = SeekFrom::Start(h_res_start_offset);

                let mut read_buf : &mut [u8] = &mut h_res_read_buf[..];
                handle.seek(seek).unwrap();
                // debug!("READ FROM {} to {} (contiguous read)", h_res_start_offset, h_res_start_offset + read_buf.len() as u64);
                handle.read(read_buf).unwrap();
            } else {
                let high_res_abs_end = h_res_archive.offset + h_res_archive.size_in_bytes;
                let first_seek = SeekFrom::Start(h_res_start_offset);
                let first_seek_bytes = high_res_abs_end - h_res_start_offset;

                // debug!("READ FROM {} to {} (wrap-around read 1)", h_res_start_offset, h_res_start_offset+first_seek_bytes);

                let (first_buf, second_buf) = h_res_read_buf.split_at_mut(first_seek_bytes as usize);

                handle.seek(first_seek).unwrap();
                handle.read(first_buf).unwrap();

                let second_seek = SeekFrom::Start(h_res_archive.offset);
                // debug!("READ FROM {} to {} (wrap-around read 2)", h_res_archive.offset, h_res_archive.offset + second_buf.len() as u64);

                handle.seek(second_seek).unwrap();
                handle.read(second_buf).unwrap();
            }

        }

        let low_res_aggregate = {
            let points : Vec<point::Point> = h_res_read_buf.chunks(point::POINT_SIZE).map(|chunk| {
                point::buf_to_point(chunk)

            }).collect();

            let timestamp_start = l_interval_start;
            let timestamp_stop = l_interval_start + (h_res_points_needed as u64)*h_res_archive.seconds_per_point;
            let step = h_res_archive.seconds_per_point;

            let expected_timestamps = range_step_inclusive(timestamp_start, timestamp_stop, step);
            let valid_points : Vec<&point::Point> = expected_timestamps.
                zip(points.iter()).
                map(|(ts, p)| {
                    if p.timestamp == ts {
                        // debug!("comparing {} and {}/{} (MATCH!)", ts, p.timestamp, p.value);
                        Some(p)
                    } else {
                        // debug!("comparing {} and {}/{}", ts, p.timestamp, p.value);
                        None
                    }
                }).filter(|agg| !agg.is_none()).map(|agg| agg.unwrap()).collect();
            self.aggregate_samples(valid_points, h_res_points_needed)
        };

        low_res_aggregate.map(|aggregate| {
            let l_res_base_point = self.read_point(l_res_archive.offset);
            let l_res_point = point::Point{ timestamp: l_interval_start, value: aggregate };
            build_write_op(l_res_archive, &l_res_point, l_res_base_point.timestamp)
        })
    }

    fn aggregate_samples(&self, points: Vec<&point::Point>, points_possible: u64) -> Option<f64>{
        // debug!("points: {:?}", points);
        let valid_points : Vec<&&point::Point> = points.iter().filter(|p| p.timestamp != 0).map(|p| p).collect();

        let ratio : f32 = valid_points.len() as f32 / points_possible as f32;
        // debug!("valid_points: {}, len: {}, ratio {} vs xff {}", valid_points.len(), points.len(), ratio, self.header.metadata.x_files_factor);
        if ratio < self.header.metadata.x_files_factor {
            // debug!("not enough data to propagate");
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

    fn split(&self, current_time: u64, point_timestamp: u64) -> Option<(&ArchiveInfo, Vec<&ArchiveInfo>)>  {
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
