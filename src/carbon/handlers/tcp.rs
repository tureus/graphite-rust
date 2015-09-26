use whisper::{ WhisperCache, NamedPoint, Schema };

use std::net::UdpSocket;
use std::io::Error;
extern crate time;

use std::sync::mpsc::{ sync_channel, SyncSender };
use std::thread;

use super::super::Config;
use super::Action;

pub fn run_server<'a>(tx: SyncSender<Action>, config: &Config) -> Result<(),Error> {
    unimplemented!();
}
