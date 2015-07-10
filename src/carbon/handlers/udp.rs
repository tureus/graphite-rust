use super::super::CarbonMsg;
use super::super::Cache;

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
