// use std::io::Error;
use std::fs::File;
use std::io::{BufWriter, SeekFrom, Cursor, Seek, Read, Write};
use byteorder::{ByteOrder, BigEndian, WriteBytesExt, ReadBytesExt};
use std::path::Path;
use std::fmt;
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
    debug!("opening file");
    let file_handle = File::open(Path::new(path));

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
    pub fn write(&mut self, current_time: u64, point: point::Point) -> Vec<WriteOp>{ 
        let pair = {
            self.split_archives(current_time, point.timestamp)
        };

        match pair {
            Some( (high_precision_archive, rest) ) => {
                let base_point = {
                    self.read_point(high_precision_archive.offset)
                };
                let base_timestamp = base_point.timestamp;
                self.calculate_write_ops( (high_precision_archive, rest) , point, base_timestamp)
            },
            None => {
                panic!("no archives satisfy current time")
            }
        }
    }

    pub fn perform_write_op(&self, write_op: &WriteOp) {
        let mut handle = self.handle.borrow_mut();
        handle.seek(write_op.seek).unwrap();
        handle.write(&(write_op.bytes)).unwrap();
    }

    fn buf_to_point(&self, buf: &[u8]) -> point::Point{
        let mut cursor = Cursor::new(buf);
        let timestamp = cursor.read_u32::<BigEndian>().unwrap() as u64;
        let value = cursor.read_f64::<BigEndian>().unwrap();
        point::Point{ timestamp: timestamp, value: value }
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
                    Ok(_) => self.buf_to_point(buf_ref),
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

    pub fn calculate_write_ops(&self, (ai,rest): (&ArchiveInfo, Vec<&ArchiveInfo>), point: point::Point, base_timestamp: u64) -> Vec<WriteOp> {
        let mut write_ops = vec![];

        {
            let write_op = build_write_op( ai, &point, base_timestamp );
            write_ops.push( write_op );
        }

        if rest.len() > 1 {
            println!("len: {:?}", rest.len());
            let low_rest_iter = rest[0..rest.len()-1].into_iter();
            let high_rest_iter = rest[1..].into_iter();
            let _ : Vec<()> = low_rest_iter.zip(high_rest_iter).map(|(l,r)|
                // WriteOps must be evaluated every time
                // because it reads the high res archive as it traverses down
                self.perform_write_op(&self.downsample(*l, *r, base_timestamp))
            ).collect();
        }


        write_ops
    }

    // The most expensive IO functionality
    // Reads many samples from high-res archive and
    // aggregates to lower-res archive. Schemas could do well to avoid
    // aggregation unless disk space is truly at a premium.
    //
    // A read-through cache would do well here. `memmap` would be awesomesauce.
    fn downsample(&self, h_res_archive: &ArchiveInfo, l_res_archive: &ArchiveInfo, base_timestamp: u64) -> WriteOp {
        let l_interval_start = l_res_archive.interval_ceiling(base_timestamp);

        let h_base_timestamp = self.read_point(h_res_archive.offset).timestamp;
        let h_res_start_offset = if h_base_timestamp == 0 {
            h_res_archive.offset
        } else {
            let timespan  = l_interval_start - h_base_timestamp;
            let points = timespan / h_res_archive.seconds_per_point;
            let bytes = points / point::POINT_SIZE as u64;
            let wrapped_index = bytes % h_res_archive.size_in_bytes;
            h_res_archive.offset + wrapped_index
        };

        let h_res_points_needed = l_res_archive.seconds_per_point / h_res_archive.seconds_per_point;
        let h_res_bytes_needed = h_res_points_needed * point::POINT_SIZE as u64;

        let h_res_end_offset = {
            let rel_first_offset = h_res_start_offset - h_res_archive.offset;
            let rel_second_offset = (rel_first_offset + h_res_bytes_needed) % h_res_archive.size_in_bytes;
            h_res_archive.offset + rel_second_offset
        };

        let mut handle = self.handle.borrow_mut();
        let mut h_res_read_buf : Vec<u8> = Vec::with_capacity(h_res_bytes_needed as usize);

        // Subroutine for filling in the buffer
        {
            // TODO: must be a better way of zeroing out a buffer to fill in the vector
            for _ in 0..h_res_bytes_needed {
                h_res_read_buf.push(0);
            }

            if h_res_start_offset < h_res_end_offset {
                // No wrap situation
                let seek = SeekFrom::Start(h_res_start_offset);
                let seek_bytes = h_res_end_offset - h_res_start_offset;

                let mut read_buf : &mut [u8] = &mut h_res_read_buf[..];
                handle.seek(seek).unwrap();
                handle.read(read_buf).unwrap();

                println!("s: {:?}, sb: {:?}, rb: {:?}", seek, seek_bytes, read_buf)
            } else {
                let first_seek = SeekFrom::Start(h_res_start_offset);
                let first_seek_bytes = h_res_end_offset - h_res_start_offset;

                let (first_buf, second_buf) = h_res_read_buf.split_at_mut(first_seek_bytes as usize);

                handle.seek(first_seek).unwrap();
                handle.read(first_buf).unwrap();

                let second_seek = SeekFrom::Start(h_res_end_offset);
                let second_seek_bytes = h_res_end_offset - h_res_archive.offset;
                handle.seek(second_seek).unwrap();
                handle.read(second_buf).unwrap();

                println!("fs: {:?}, fsb: {:?}, ss: {:?}, ssb: {:?}", first_seek, first_seek_bytes, second_seek, second_seek_bytes);
            }
        }

        let low_res_aggregate = {
            let points : Vec<point::Point> = h_res_read_buf.chunks(point::POINT_SIZE).map(|chunk|
                self.buf_to_point(chunk)
            ).collect();
            self.aggregate_samples(points)
        };

        {
            let l_res_base_point = self.read_point(l_res_archive.offset);
            let l_res_point = point::Point{ timestamp: l_interval_start, value: low_res_aggregate };
            build_write_op(l_res_archive, &l_res_point, l_res_base_point.timestamp)
        }

    }

    fn aggregate_samples(&self, points: Vec<point::Point>) -> f64{
        // TODO: we only do aggregation right now!
        // TODO: need to use the xff property and turn this in to a Option
        match self.header.metadata.aggregation_type {
            AggregationType::Average => {
                let sum = points.iter().map(|p| p.value).fold(0.0, |l, r| l + r);
                sum / points.len() as f64
            },
            _ => { 0.0 }
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

    let point_value = point.value;
    {
        let buf : &mut [u8] = &mut output_data;
        let mut writer = BufWriter::new(buf);
        writer.write_u32::<BigEndian>(interval_ceiling as u32).unwrap();
        writer.write_f64::<BigEndian>(point_value).unwrap();
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
        // WriteOp{seek: SeekFrom::Start(56), bytes: [0,0,0,0,0,0,0,0,0,0,0,0]}
    ];
    assert_eq!(write_ops.len(), expected.len());
    assert_eq!(write_ops, expected);

    return;
}
