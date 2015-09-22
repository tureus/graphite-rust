#![feature(unmarked_api)]

extern crate graphite;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate time;

use graphite::carbon;

use std::path::Path;

use docopt::Docopt;
static USAGE: &'static str = "
Carbon is the network service for writing data to disk

Usage:
  carbon [--port PORT] [--bind HOST] [--chan DEPTH] [--storage-path STORAGEPATH] [--cache-size CACHESIZE]
  carbon --help

Options:
  -h --help                   show this screen
  --bind HOST                 host to bind to [default: 0.0.0.0:2003]
  --chan DEPTH                how many carbon messages can be in-flight [default: 1000]
  --storage-path STORAGEPATH  where to find the whisper file [default: /tmp]
  --cache-size CACHESIZE      max number of open files to keep in memory [default: 60000]
";

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_bind: String,
    flag_chan: usize,
    flag_storage_path: String,
    flag_cache_size: usize
}

pub fn main(){
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let bind_spec = unsafe {
        args.flag_bind.slice_unchecked(0, args.flag_bind.len())
    };

    let config = carbon::Config{
        bind_spec: bind_spec,
        chan_depth: args.flag_chan,
        base_path: Path::new(&args.flag_storage_path),
        cache_size: args.flag_cache_size
    };

    info!("starting carbon server...");
    carbon::udp::run_server(&config).unwrap();
}
