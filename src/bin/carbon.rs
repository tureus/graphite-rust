#![feature(test)]
#![feature(unmarked_api)]

extern crate graphite;

#[macro_use]
extern crate log;
extern crate env_logger;
extern crate rustc_serialize;
extern crate docopt;
extern crate time;

use graphite::whisper::{ Point };
use graphite::carbon;

use std::path::PathBuf;

use docopt::Docopt;
static USAGE: &'static str = "
Carbon is the network service for writing data to disk

Usage: carbon [--port PORT] [--bind HOST] [--chan DEPTH]

Options:
    --bind HOST    host to bind to [default: 0.0.0.0:2003]
    --chan DEPTH   how many carbon messages can be in-flight [default: 1000]
";

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_bind: String,
    flag_chan: usize
}

pub fn main(){
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let bind_spec = unsafe {
        args.flag_bind.slice_unchecked(0, args.flag_bind.len())
    };

    let chan_depth = args.flag_chan;

    info!("starting carbon server...");

    carbon::udp::run_server(bind_spec, chan_depth).unwrap();
}


#[cfg(test)]
mod tests {
    #![feature(test)]

    extern crate test;
    use self::test::Bencher;

    extern crate graphite;
    use graphite::whisper::Point;

    use graphite::carbon::CarbonMsg;
    use std::path::Path;

    #[bench]
    fn bench_good_datagram(b: &mut Bencher){
        let datagram = "home.pets.bears.lua.purr_volume 100.00 1434598525\n";

        b.iter(|| {
            let msg_opt = CarbonMsg::from_datagram(datagram.as_bytes());
            msg_opt.unwrap();
        });
    }

    #[test]
    fn test_good_datagram() {
        let datagram = "home.pets.bears.lua.purr_volume 100.00 1434598525\n";
        let msg_opt = CarbonMsg::from_datagram(datagram.as_bytes());
        let msg = msg_opt.unwrap();

        let expected = CarbonMsg {
            metric_rel_path: Path::new("home/pets/bears/lua/purr_volume.wsp").to_path_buf(),
            point: Point {
                value: 100.0,
                timestamp: 1434598525
            }
        };

        assert_eq!(msg, expected);
    }

    #[bench]
    fn bench_bad_datagram(b: &mut Bencher){
        let datagram = "home.pets.monkeys.squeeky.squeeks asdf 1434598525\n";

        b.iter(|| {
            let msg_opt = CarbonMsg::from_datagram(datagram.as_bytes());
            assert!(msg_opt.is_err());
        });
    }
}
