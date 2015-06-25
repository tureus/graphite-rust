#![feature(collections)]
#![feature(path_ext)]
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

#[derive(Debug, PartialEq)]
struct CarbonMsg {
    pub metric_rel_path: PathBuf,
    point: Point
}

#[derive(Debug)]
enum Error {
    BadDatagram
}

#[derive(Debug)]
struct CarbonError(Error,String);

impl CarbonMsg {
    pub fn from_datagram(datagram_buffer: &[u8]) -> Result<CarbonMsg, CarbonError> {
        let datagram = match str::from_utf8(datagram_buffer) {
            Ok(body) => body,
            Err(_) => return Err( CarbonError(Error::BadDatagram, "invalid utf8 character".to_string() ))
        };

        // TODO: this seems too complicated just to detect/remove "\n"
        // And it scans the string as utf8 codepoints.
        // Will this just be ASCII... is it safe to skip utf8-ness? (probs not)
        let (without_newline,newline_str) = {
            let len = datagram.len();
            ( 
              datagram.slice_chars(0, len-1),
              datagram.slice_chars(len-1, len)
            )
        };
        if newline_str != "\n" {
            return Err( CarbonError(Error::BadDatagram, format!("Datagram `{}` is missing a newline `{}`", datagram, newline_str)))
        }

        let parts : Vec<&str> = without_newline.split(" ").collect();
        if parts.len() != 3 {
            return Err( CarbonError(Error::BadDatagram, format!("Datagram `{}` does not have 3 parts", datagram) ) );
        }

        // TODO: copies to msg. Used to be a reference from datagram_buffer
        // but figuring out how to keep the datagram_buffer (which was on the heap)
        // alive long enough was tricky.
        let metric_name = parts[0].to_string();
        let mut rel_path : String = metric_name.replace(".","/");
        rel_path.push_str(".wsp");

        let value = {
            let value_parse = parts[1].parse::<f64>();
            match value_parse {
                Ok(val) => val,
                Err(_) => {
                    return Err( CarbonError(Error::BadDatagram, format!("Datagram value `{}` is not a float", parts[1]) ) )
                }
            }
        };

        let timestamp = {
            let timestamp_parse = parts[2].parse::<u64>();
            match timestamp_parse {
                Ok(val) => val,
                Err(_) => {
                    return Err( CarbonError(Error::BadDatagram, format!("Datagram value `{}` is not an unsigned integer", parts[2])) )
                }
            }
        };

        let msg = CarbonMsg {
            metric_rel_path: PathBuf::from(rel_path),
            point: Point {
                value: value,
                timestamp: timestamp                
            }
        };
        Ok(msg)

    }
}

mod carbon {
    pub mod udp {
        use super::super::CarbonMsg;
        use super::cache::Cache;

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

    // The cache models the whisper files on the filesystem
    // It handles putting all datapoints in to the files
    pub mod cache {
        use super::super::CarbonMsg;
        use graphite::whisper::WhisperFile;
        use graphite::whisper::schema::Schema;
        use std::collections::HashMap;
        use std::path::{ Path, PathBuf };
        use std::fs::{ PathExt, DirBuilder };
        use std::io;

        // #[derive(Debug)]
        // pub enum CacheErrorType {
        //     BadBasePath
        // }

        // #[derive(Debug)]
        // pub struct CacheError {
        //     reason: CacheErrorType
        // }

        #[derive(Debug)]
        pub struct Cache {
            base_path: PathBuf,
            open_files: HashMap<PathBuf, WhisperFile>
        }

        impl Cache {
            pub fn new(base_path: &Path) -> Cache {
                Cache {
                    base_path: base_path.to_path_buf(),
                    open_files: HashMap::new()
                }
            }

            pub fn write(&mut self, current_time: u64, incoming: CarbonMsg) -> Result<(), io::Error> {

                let mut whisper_file = try!( self.resolve(incoming.metric_rel_path) );
                whisper_file.write(current_time, incoming.point);
                Ok(())

            }

            // Find or initialize the whisper file
            fn resolve(&mut self, metric_rel_path: PathBuf) -> Result<&mut WhisperFile, io::Error> {

                if self.open_files.contains_key(&metric_rel_path) {

                    debug!("file cache hit. resolved {:?}", metric_rel_path);
                    Ok( self.open_files.get_mut(&metric_rel_path).unwrap() )

                } else {

                    debug!("file cache miss. resolving {:?}", metric_rel_path);

                    let path_for_insert = metric_rel_path.clone();
                    let path_for_relookup = metric_rel_path.clone();

                    let path_on_disk = self.base_path.join(metric_rel_path);

                    let whisper_file = if path_on_disk.exists() && path_on_disk.is_file() {

                        debug!("`{:?}` exists on disk. opening.", path_on_disk);
                        // TODO: might make sense push this logic to instantiation of CarbonMsg
                        // TODO: shouldn't be a UTF8 error cuz of std::str::from_utf8() in CarbonMsg input
                        try!( WhisperFile::open(&path_on_disk) )

                    } else {

                        debug!("`{:?}` file does not exist on disk. creating default.", path_on_disk);
                        let default_specs = vec!["1s:60s".to_string(), "1m:1y".to_string()];
                        let schema = Schema::new_from_retention_specs(default_specs);

                        // Verify the folder structure is present.
                        // TODO: benchmark (for my own curiosity)
                        // TODO: assumption here is that we do not store in root FS
                        if !path_on_disk.parent().unwrap().is_dir() {
                            debug!("`{:?}` must be created first", path_on_disk.parent());
                            try!( DirBuilder::new().recursive(true).create( path_on_disk.parent().unwrap() ) );
                        }
                        try!( WhisperFile::new(&path_on_disk, schema) )

                    };

                    self.open_files.insert(path_for_insert, whisper_file);
                    Ok( self.open_files.get_mut(&path_for_relookup).unwrap() )

                }

            }
        }

        #[cfg(test)]
        mod test {
            extern crate test;
            extern crate graphite;

            use self::test::Bencher;

            use std::path::{ Path };

            use super::Cache;
            use super::super::super::CarbonMsg;

            use graphite::whisper::Point;

            #[bench]
            fn test_opening_new_whisper_file(b: &mut Bencher){
                let mut cache = Cache::new(Path::new("/tmp"));
                let current_time = 1434598525;

                b.iter(move ||{

                    let metric = CarbonMsg {
                        metric_rel_path: Path::new("hey/there/bear.wsp").to_path_buf(),
                        point: Point {
                            value: 0.0,
                            timestamp: 1434598525
                        }
                    };

                    cache.write(current_time, metric).unwrap();

                });
            }
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

    use super::CarbonMsg;
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
