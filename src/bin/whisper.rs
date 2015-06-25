extern crate graphite;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate time;

use docopt::Docopt;
use graphite::whisper::{ WhisperFile, Point };
use graphite::whisper::schema::Schema;

use std::path::Path;

static USAGE: &'static str = "
Whisper is the fast file manipulator

Usage:
    whisper info <file>
    whisper dump <file>
    whisper update <file> <timestamp> <value>
    whisper mark <file> <value>
    whisper thrash <file> <value> <times>
    whisper create <file> <timespec>...

Options:
    --xff <x_files_factor>
    --aggregation_method <method>
";

#[derive(RustcDecodable, Debug)]
struct Args {
    cmd_info: bool,
    cmd_dump: bool,
    cmd_update: bool,
    cmd_mark: bool,
    cmd_thrash: bool,
    cmd_create: bool,

    arg_file: String,
    arg_timestamp: String,
    arg_value: String,
    arg_times: String,

    arg_timespec: Vec<String>
}


pub fn main(){
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let arg_file = args.arg_file.clone();
    let path_str : &str = unsafe {
        arg_file.slice_unchecked(0, args.arg_file.len())
    };
    let path = Path::new(path_str);


    let current_time = time::get_time().sec as u64;

    if args.cmd_info {
        cmd_info(path);
    } else if args.cmd_dump {
        cmd_dump(path);
    } else if args.cmd_update {
        cmd_update(args, path, current_time);
    } else if args.cmd_mark {
        cmd_mark(args, path, current_time);
    } else if args.cmd_thrash {
        cmd_thrash(args, path, current_time);
    } else if args.cmd_create {
        cmd_create(args, path);
    } else {
        println!("Must specify command.");
    }
}

fn cmd_info(path: &Path) {
    let file_open = WhisperFile::open(path);
    match file_open {
        Ok(whisper_file) => println!("{}", whisper_file),
        Err(why) => {
            println!("could create whisper file: {:?}", why)
        }
    }
}

fn cmd_dump(path: &Path) {
    let file_open = WhisperFile::open(path);
    match file_open {
        Ok(whisper_file) => println!("{:?}", whisper_file),
        Err(why) => {
            println!("could create whisper file: {:?}", why)
        }
    }
}

fn cmd_update(args: Args, path: &Path, current_time: u64) {
    let mut file = WhisperFile::open(path).unwrap();
    let point = Point{
        timestamp: args.arg_timestamp.parse::<u64>().unwrap(),
        value: args.arg_value.parse::<f64>().unwrap()
    };
    debug!("Updating TS: {} with value: {}", point.timestamp, point.value);

    file.write(current_time, point);
}

fn cmd_mark(args: Args, path: &Path, current_time: u64) {
    let mut file = WhisperFile::open(path).unwrap();
    let point = Point{
        timestamp: current_time,
        value: args.arg_value.parse::<f64>().unwrap()
    };

    file.write(current_time, point);
}

fn cmd_thrash(args: Args, path: &Path, current_time: u64) {
    let times = args.arg_times.parse::<u64>().unwrap();
    let mut file = WhisperFile::open(path).unwrap();
    for index in 1..times {
        let point = Point{
            timestamp: current_time+index,
            value: args.arg_value.parse::<f64>().unwrap()
        };

        file.write(current_time+index, point);
    }
}

fn cmd_create(args: Args, path: &Path) {
    let schema = Schema::new_from_retention_specs(args.arg_timespec);
    let new_result = WhisperFile::new(path, schema);
    match new_result {
        Ok(whisper_file) => println!("Success! {}", whisper_file),
        Err(why) => println!("Failed: {:?}", why)
    }
}
