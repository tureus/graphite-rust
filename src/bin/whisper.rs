extern crate graphite;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate time;

use docopt::Docopt;
use graphite::whisper;

static USAGE: &'static str = "
Usage:
    whisper info <file>
    whisper update <file> <timestamp> <value>
    whisper mark <file> <value>
    whisper thrash <file> <value> <times>
";

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_file: String,

    cmd_info: bool,

    cmd_update: bool,
    cmd_mark: bool,
    arg_timestamp: String,
    arg_value: String,

    cmd_thrash: bool,
    arg_times: String
}


pub fn main(){
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let path = unsafe {
        args.arg_file.slice_unchecked(0, args.arg_file.len())
    };

    let current_time = time::get_time().sec as u64;

    if args.cmd_info {
        let file = whisper::file::open(path).unwrap();
        println!("{:?}", file);
    } else if args.cmd_update {
        let mut file = whisper::file::open(path).unwrap();
        let point = whisper::point::Point{
            timestamp: args.arg_timestamp.parse::<u64>().unwrap(),
            value: args.arg_value.parse::<f64>().unwrap()
        };
        debug!("Updating TS: {} with value: {}", point.timestamp, point.value);

        file.write(current_time, point);
    } else if args.cmd_mark {
        let mut file = whisper::file::open(path).unwrap();
        let point = whisper::point::Point{
            timestamp: current_time,
            value: args.arg_value.parse::<f64>().unwrap()
        };

        file.write(current_time, point);
    } else if args.cmd_thrash {
        let times = args.arg_times.parse::<u64>().unwrap();
        let mut file = whisper::file::open(path).unwrap();
        for index in 1..times {
            let point = whisper::point::Point{
                timestamp: current_time+index,
                value: args.arg_value.parse::<f64>().unwrap()
            };

            file.write(current_time+index, point);
        }
    }
}
