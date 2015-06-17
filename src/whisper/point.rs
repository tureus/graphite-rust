use std::io::{ BufWriter, Cursor };
use byteorder::{ ByteOrder, BigEndian, ReadBytesExt, WriteBytesExt };
use super::file::BucketName;

#[derive(PartialEq,Debug,Clone)]
pub struct Point {
    pub timestamp: u64,
    pub value: f64
}

// TODO: generate this from the struct definition?
pub const POINT_SIZE : usize = 12;

#[inline]
pub fn buf_to_point(buf: &[u8]) -> Point {
    let mut cursor = Cursor::new(buf);
    let timestamp = cursor.read_u32::<BigEndian>().unwrap() as u64;
    let value = cursor.read_f64::<BigEndian>().unwrap();
    Point{ timestamp: timestamp, value: value }
}

#[inline]
pub fn fill_buf(buf: &mut [u8], bucket: BucketName, point_value: f64) {
    let mut writer = BufWriter::new(buf);
    writer.write_u32::<BigEndian>(bucket.0 as u32).unwrap();
    writer.write_f64::<BigEndian>(point_value).unwrap();
}
