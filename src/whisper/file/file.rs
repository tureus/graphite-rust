// use std::io::Error;
use std::fs::File;
use std::path::Path;

use super::header::{ Header, read_header };
use super::write_op::{ WriteOp };

use super::metadata::{ Metadata, AggregationType };
#[allow(dead_code)]
use super::archive_info::{ ArchiveInfo };
use whisper::point;

use time::{ get_time };

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
        let hp_ai_option = self.header.archive_infos.iter().find(|ai|
            (current_time - point.time) < ai.retention
        );

        let write_ops = vec![];

        match hp_ai_option {
            Some(hp) => {
                let op = WriteOp{

                };
                write_ops.push(op)
                write_ops
            }
            None => {
                write_ops
            }
        }

        // return self.header.archive_infos.iter().map(|ai| ai.calculate_write_op(&point) ).collect();
    }

    pub fn read(&self) -> point::Point {
        point::Point{value: 10.0, time: 10}
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
        point::Point{value: 0.0, time: 10}
    );

    let expected = vec![
        WriteOp{offset: 28, value: 0.0},
        WriteOp{offset: 56, value: 0.0}
    ];
    assert_eq!(write_ops, expected);

    return;
}
