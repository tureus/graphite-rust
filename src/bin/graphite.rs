extern crate graphite;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate time;

use std::path::Path;

use docopt::Docopt;
static USAGE: &'static str = "
Graphite is the HTTP REST API for querying data from the database

Usage:
    graphite server
    graphite expand [--storage-path STORAGEPATH] <pattern>
Options:

    --bind HOST                 host to bind to [default: 0.0.0.0:8080]
    --storage-path STORAGEPATH  where to find the whisper file [default: /tmp]
";

use self::graphite::graphite::{ Config, server, expander };
use self::graphite::whisper::Cache;

#[derive(RustcDecodable, Debug)]
struct Args {
    cmd_server: bool,
    cmd_expand: bool,

    arg_pattern: String,

    flag_bind: String,
    flag_storage_path: String
}

pub fn main(){
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let bind_spec : &str = unsafe {
        args.flag_bind.slice_unchecked(0, args.flag_bind.len())
    };

    let config = Config{
        bind_spec: bind_spec,
        base_path: Path::new(&args.flag_storage_path)
    };

    if args.cmd_server {
        let cache = Cache::new(config.base_path);
        server::run(config, cache);
    } else if args.cmd_expand {
        let cache = Cache::new(config.base_path);
        expander::expand(&args.arg_pattern, &cache);
    } else {
        println!("command not specified");
    }
}
