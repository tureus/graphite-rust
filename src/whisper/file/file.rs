// use std::io::Error;
use std::fs::File;
use std::io::{BufWriter};
use byteorder::{ByteOrder, BigEndian, WriteBytesExt};
use std::path::Path;

use super::header::{ Header, read_header };
use super::write_op::{ WriteOp };

#[allow(dead_code)]
use super::archive_info::{ ArchiveInfo };
use whisper::point;

#[derive(Debug)]
pub struct WhisperFile<'a> {
    pub path: &'a str,
    pub header: Header
}

// TODO: Change error value to generic Error
pub fn open(path:& str) -> Result<WhisperFile, &'static str> {
    debug!("opening file");
    let file_handle = File::open(Path::new(path));

    match file_handle {
        Ok(f) => {
            let header = try!(read_header(f));

            Ok(
                WhisperFile {
                    path: path,
                    header: header
                }
            )
        },
        Err(_) => {
            Err("generic file error")
        }
    }
}

impl<'a> WhisperFile<'a> {
    // The header information is not thread safe,
    // other processes may open the file and modify the contents
    // which would be bad. It's your job, the caller, to make sure
    // the system can't do that.
    pub fn write(&self, point: point::Point){ 
        // get archive
        // calculate archive data offset + point offset
        // 
        debug!("writing point: {:?}", point);
    }

    pub fn calculate_write_ops(&self, current_time: u32, point: point::Point) -> Vec<WriteOp> {
        // let current_time = get_time().sec as u32;
        let mut archive_iter = self.header.archive_infos.iter();
        
        let hp_ai_option = archive_iter.find(|ai|
            (current_time - point.timestamp) < ai.retention as u32
        ); 
        let mut rest_of_archives = archive_iter;

        let low_res_archives : Vec<&ArchiveInfo> = rest_of_archives.collect();

        let write_ops = vec![];
        match hp_ai_option {
            Some(ai) => {
                write_ops
            }
            None => {
                write_ops
            }
        }
    }

    pub fn read(&self) -> point::Point {
        point::Point{value: 10.0, timestamp: 10}
    }
}

fn build_write_op(current_time: u32, archive_info: ArchiveInfo, point: point::Point, base_point: point::Point) -> WriteOp {
    let mut output_data = [0; 12];
    let interval_ceiling = archive_info.interval_ceiling(&point);

    let point_value = point.value;
    {
        let mut buf : &mut [u8] = &mut output_data;
        let mut writer = BufWriter::new(buf);
        writer.write_u32::<BigEndian>(interval_ceiling).unwrap();
        writer.write_f64::<BigEndian>(point_value);
    }

    let seek_info = archive_info.calculate_seek(&point, &base_point);

    return WriteOp {
        seek: seek_info,
        bytes: output_data
    }
}


#[test]
fn has_write_ops(){
    let whisper_file = WhisperFile{
        path: "/a/nonsense/path",
        header: Header {
            metadata: Metadata {
                aggregation_type: AggregationType::Average,
                max_retention: 86400,
                x_files_factor: 1056964608,
                archive_count: 1
            },
            archive_infos: vec![
                ArchiveInfo {
                    offset: 28,
                    seconds_per_point: 60,
                    points: 1440,
                    retention: 60 * 1440
                },
                ArchiveInfo {
                    offset: 56,
                    seconds_per_point: 60,
                    points: 1440,
                    retention: 60 * 1440
                }
            ]
        }
    };

    let fixture_time = 20;
    let write_ops = whisper_file.calculate_write_ops(
        fixture_time,
        point::Point{value: 0.0, timestamp: 10}
    );

    let expected = vec![
        WriteOp{offset: 28, value: 0.0},
        WriteOp{offset: 56, value: 0.0}
    ];
    assert_eq!(write_ops, expected);

    return;
}
