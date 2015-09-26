use whisper::{ WhisperCache };

use std::thread::{ self, JoinHandle };
// extern crate time;
use std::sync::mpsc::{ sync_channel, SyncSender };

use super::Config;
use super::handlers::Action;

pub fn spawn(cache: WhisperCache, config: &Config) -> (SyncSender<Action>, JoinHandle<()>) {
    let (tx, rx) = sync_channel(config.chan_depth);

    info!("spawning file writer...");
    let mut cache = cache; // gotta alias the value as mut!

    let writer = thread::spawn(move || {
        loop {
            let recv = rx.recv();
            // let current_time = time::get_time().sec as u64;

            match recv {
                Ok(Action::Write(named_point)) => {
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

    (tx,writer)
}
