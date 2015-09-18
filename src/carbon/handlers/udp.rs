use whisper::{ WhisperCache, NamedPoint, Schema };

use std::net::UdpSocket;
use std::io::Error;
extern crate time;

use std::sync::mpsc::sync_channel;
use std::sync::{ RwLock };
use std::thread;

use super::config::Config;


pub fn run_server<'a>(config: &Config) -> Result<(),Error> {
    let (tx, rx) = sync_channel(config.chan_depth);

    // Why can't I just `clone()` the base path?
    let default_specs = vec!["1s:60s".to_string(), "1m:1y".to_string()];
    let schema = Schema::new_from_retention_specs(default_specs);
    let raw_cache = WhisperCache::new(&config.base_path.to_owned(), schema);
    let locked_cache = RwLock::new(raw_cache);

    info!("spawning file writer...");
    thread::spawn(move || {
        loop {
            let recv = rx.recv();
            // let current_time = time::get_time().sec as u64;

            match recv {
                Ok(named_point) => {
                    let mut cache = locked_cache.write().unwrap();
                    let write_res = cache.write( named_point );

                    match write_res {
                        Ok(()) => (),
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

    info!("server binding to `{:?}`", config.bind_spec);
    let mut buf_box = create_buffer();
    let socket = try!( UdpSocket::bind(config.bind_spec) );
    loop {
        let (bytes_read,_) = {
            match socket.recv_from( &mut buf_box[..] ) {
                Ok(res) => {
                    res
                }
                Err(err) => {
                    error!("error reading from socket: {:?}", err);
                    continue;
                }
            }
        };

        match NamedPoint::from_datagram(&buf_box[0..bytes_read]) {
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
