use whisper::{ NamedPoint };

use std::net::{ TcpListener, TcpStream };
use std::io::{ Error, BufReader, BufRead };
extern crate time;

use std::sync::mpsc::{ sync_channel, SyncSender };
use std::thread::{ self, JoinHandle };

use super::super::Config;
use super::Action;

pub fn run_server(tx: SyncSender<Action>, config: &Config) -> Result<JoinHandle<Result<(),Error>>,Error> {
    let listener = try!( TcpListener::bind(config.bind_spec) );

    let listener_tx = tx.clone();
    let accept_thread = thread::spawn(move ||{
        let listener_tx = listener_tx;

        for listen_result in listener.incoming() {
            let tcp_stream = try!(listen_result);
            let thread_tx = listener_tx.clone();
            thread::spawn(move || {
                do_server(thread_tx, tcp_stream);
            });
        };
        Ok(())
    });

    Ok(accept_thread)
}

fn do_server(tx: SyncSender<Action>, tcp_stream: TcpStream) {
    let mut line_buf = String::new();
    let mut reader = BufReader::new(tcp_stream);

    loop {
        match reader.read_line(&mut line_buf) {
            Ok(bytes_read) => {
                debug!("tcp listener read {} bytes", bytes_read);

                let parsed_line = NamedPoint::parse_line(&line_buf[..]);
                match parsed_line {
                    Ok(np) => tx.send(Action::Write(np)).unwrap(),
                    Err(err) => {
                        error!("could not parse incoming data: {:?}", err);
                        break;
                    }
                }
            },
            Err(err) => {
                info!("shutting down tcp listener: {:?}", err);
                break
            }
        };
    }
}
