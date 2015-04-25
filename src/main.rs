extern crate time;
extern crate byteorder;

pub mod whisper;

pub fn main(){
    let test_point = whisper::point::Point{value: 0.0, time: 1000};
    write_test_point(test_point);

    return;
}

pub fn write_test_point(point: whisper::point::Point){
    let path = "./test/fixtures/60-1440.wsp";
    let open_result = whisper::file::open(path);

    match open_result {
        Ok(f) => f.write(point),
        Err(e) => println!("no file for reading! {}", e)
    }
}
