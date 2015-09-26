use whisper::NamedPoint;

pub mod udp;
pub mod tcp;

// Room to add functionality such as
// - USR1 signal print state of cache to STDOUT
// - USR2 signal flush state of cache to DISK
pub enum Action {
    Write(NamedPoint)
}
