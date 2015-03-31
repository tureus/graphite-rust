extern crate byteorder;

pub mod whisper;

pub fn main(){
    read_header();

    let point = whisper::point::Point{time: 0, value: 1.3 };
    println!("point: {:?}", point);

    return;
}

pub fn read_header(){
    let path = "./test/fixtures/60-1440.wsp";
    let open_result = whisper::file::open(path);

    match open_result {
        Ok(f) => {
            // let header = whisper::header::read_header(f);
            println!("header: {:?}", f);

            let test_point = whisper::point::Point{time: 0, value: 0.0};
            f.write(test_point);
        }
        Err(e) => {
            println!("no file for reading! {}", e);
        }
    }
}
