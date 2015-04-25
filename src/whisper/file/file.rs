// use std::io::Error;
use std::fs::File;
use std::path::Path;

use super::header;
use super::write_op::{WriteOp};
use whisper::point;

#[derive(Debug)]
pub struct WhisperFile<'a> {
    pub path: &'a str,
    pub header: header::Header
}

// TODO: Change error value to generic Error
pub fn open(path:& str) -> Result<WhisperFile, &'static str> {
    let file_handle = File::open(Path::new(path));

    match file_handle {
        Ok(f) => {
            let header = try!(header::read_header(f));

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
        println!("file: {:?}\npoint: {:?}", self, point);
    }

    pub fn calculate_write_ops(&self, point: point::Point) -> Vec<WriteOp> {
        return self.header.archive_infos.iter().map(|ai| ai.write(&point) ).collect();
    }

    pub fn read(&self, timestamp: u32) -> point::Point {
        point::Point{value: 10.0, time: 10}
    }
}


#[test]
fn writes_to_a_file(){

}
