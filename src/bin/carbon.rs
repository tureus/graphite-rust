#![feature(collections)]
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

use std::str;
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

mod carbon {
    pub mod udp {
        use graphite::carbon::CarbonMsg;
        use graphite::carbon::Cache;

        use std::net::UdpSocket;
        use std::io::Error;
        extern crate time;

        use std::sync::mpsc::sync_channel;
        use std::thread;

        use std::path::Path;

        pub fn run_server(bind_spec: &str, chan_depth: usize) -> Result<(),Error> {
            let (tx, rx) = sync_channel(chan_depth);

            let base_path = Path::new("/Users/xavierlange/code/rust/graphite-rust/test/fixtures");
            let mut cache = Cache::new(base_path.clone());

            info!("spawning file writer...");
            thread::spawn(move || {
                loop {
                    let recv = rx.recv();
                    let current_time = time::get_time().sec as u64;

                    match recv {
                        Ok(msg) => {
                            match cache.write(current_time, msg) {
                                Ok(_) => (),
                                Err(reason) => debug!("err: {:?}", reason)
                            }
                        },
                        Err(_) => {
                            debug!("shutting down writer thread");
                            return ()
                        }
                    }
                }
            });

            info!("server binding to `{}`", bind_spec);
            let mut buf_box = create_buffer();
            let socket = try!( UdpSocket::bind(bind_spec) );
            loop {
                let (bytes_read,_) = {
                    match socket.recv_from( &mut buf_box[..] ) {
                        Ok(res) => res,
                        Err(err) => {
                            debug!("error reading from socket: {:?}", err);
                            continue;
                        }
                    }
                };

                match CarbonMsg::from_datagram(&buf_box[0..bytes_read]) {
                    Ok(msg) => {
                        // Dies if the receiver is closed
                        tx.send(msg).unwrap();
                    },
                    Err(err) => {
                        debug!("wtf mate: {:?}", err);
                    }
                };
            }
        }

        fn create_buffer() -> Box<[u8]> {
            let buf : [u8; 8*1024] = [0; 8*1024];
            Box::new( buf )
        }
    }
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
