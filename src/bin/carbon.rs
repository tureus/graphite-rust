extern crate graphite;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;

use graphite::whisper;

use docopt::Docopt;
static USAGE: &'static str = "
Carbon is the network service for writing data to disk

Usage: carbon [--port PORT] [--bind HOST]

Options:
    --port PORT   port to listen on [default: 2003]
    --bind HOST   host to bind to [default: localhost]
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_port: u64,
    arg_bind: Option<String>
}

pub fn main(){
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    println!("args: {:?}", args);
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
