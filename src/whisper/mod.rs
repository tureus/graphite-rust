/*!

All the code necessary for creating, reading, and updating
whisper database files. The 'standard' for whisper files is defined
primarily from the source code. It's a straight-forward format for
static allocation of time-series

*/

extern crate byteorder;

pub mod file;
pub mod point;
pub mod schema;
