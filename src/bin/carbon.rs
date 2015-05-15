extern crate graphite;

#[macro_use]
extern crate log;
extern crate env_logger;

use graphite::whisper;

pub fn main(){
    env_logger::init().unwrap();

    let test_point = whisper::point::Point{value: 0.0, timestamp: 1000};
    
    write_test_point(test_point);

    return;
}

pub fn write_test_point(point: whisper::point::Point){
    let path = "./test/fixtures/60-1440-1440-168-10080-52.wsp";
    let open_result = whisper::file::open(path);

    match open_result {
        Ok(mut f) => {
            f.write(1001, point);
            return
        },
        Err(e) => error!("no file for reading! {}", e)
    }
}
