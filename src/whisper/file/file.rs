// use std::io::Error;
use std::fs::File;
use std::path::Path;

use super::header::{Header, read_header};
use super::metadata;
use super::archive_info;
use super::write_op::{WriteOp};
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

    pub fn calculate_write_ops(&self, point: point::Point) -> Vec<WriteOp> {
        return self.header.archive_infos.iter().map(|ai| ai.calculate_write_op(&point) ).collect();
    }

    pub fn read(&self, timestamp: u32) -> point::Point {
        point::Point{value: 10.0, time: 10}
    }
}


#[test]
fn has_write_ops(){
    let whisper_file = WhisperFile{
        path: "/a/nonsense/path",
        header: Header {
            metadata: metadata::Metadata {
                aggregation_type: metadata::AggregationType::Average,
                max_retention: 86400,
                x_files_factor: 1056964608,
                archive_count: 1
            },
            archive_infos: vec![
                archive_info::ArchiveInfo {
                    offset: 28,
                    seconds_per_point: 60,
                    points: 1440
                },
                archive_info::ArchiveInfo {
                    offset: 56,
                    seconds_per_point: 60,
                    points: 1440
                }
            ]
        }
    };

    let write_ops = whisper_file.calculate_write_ops(
        point::Point{value: 0.0, time: 10}
    );

    let expected = vec![
        WriteOp{offset: 28, value: 0.0},
        WriteOp{offset: 56, value: 0.0}
    ];
    assert_eq!(write_ops, expected);

    return;
}
