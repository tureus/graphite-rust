// use std::io::Error;
use std::fs::File;
use std::path::Path;

use super::header;
use whisper::point;

#[derive(Debug)]
pub struct WhisperFile<'a> {
    pub path: &'a str,
    pub header: header::Header
}

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
    pub fn write(&self, point: point::Point){
        println!("file: {:?}, point: {:?}", self, point);
    }
}
