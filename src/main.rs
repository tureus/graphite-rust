#![feature(fs,path,io)]
extern crate byteorder;

pub mod whisper;

use std::io::Write;

fn main(){
  read_header();
  return;
}

pub fn read_header(){
  let path = "./test/fixtures/60-1440.wsp";
  let open_result = whisper::file::open(path);

  match open_result {
    Ok(f) => {
      let header = whisper::header::read_header(f);
      println!("header: {:?}", header);
    }
    Err(e) => {
      println!("no file for reading! {}", e);
    }
  }
}

pub fn test_allocation(){
  let allocation_result = whisper::file::allocate("./a_file.wsp");
  match allocation_result {
    Ok(mut f) => {
      let result = f.write_all(b"Hello, world!");
      match result {
        Ok(_) => println!("yay, write was successful"),
        Err(e) => println!("write failed with {}", e)
      }
    },
    Err(e) => println!("sorry, got {}",e),
  }
}