#![feature(fs,path,io)]
extern crate byteorder;

pub mod whisper;

pub fn main(){
    return;
}

pub fn read_header(){
    let path = "./test/fixtures/60-1440.wsp";
    let open_result = whisper::file::open(path);

    match open_result {
        Ok(f) => {
            // let header = whisper::header::read_header(f);
            println!("header: {:?}", f.header);
        }
        Err(e) => {
            println!("no file for reading! {}", e);
        }
    }
}
