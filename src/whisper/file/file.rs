use std::io::Error;
use std::fs::File;
use std::path::Path;

use super::header;

pub struct WhisperFile<'a> {
    pub path: &'a str,
    pub header: header::Header
}

pub fn open(path:& str) -> Result<WhisperFile, Error> {
    let file_handle = File::open(Path::new(path));

    match file_handle {
        Ok(f) => {
            let header = header::read_header(f);
            Ok(
                WhisperFile {
                    path: path,
                    header: header
                }
            )
        },
        Err(e) => {
            Err(e)
        }
    }
}
