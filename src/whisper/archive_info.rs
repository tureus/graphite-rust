use std::io::Cursor;
use byteorder::{BigEndian, ReadBytesExt};

#[derive(PartialEq,Debug)]
pub struct ArchiveInfo {
    pub offset: u32,
    pub seconds_per_point: u32,
    pub points: u32
}

pub fn slice_to_archive_info(buf: &[u8]) -> ArchiveInfo{
    println!("read {:?}", buf);

    let mut cursor = Cursor::new(buf);
    let offset = cursor.read_u32::<BigEndian>().unwrap();
    let seconds_per_point = cursor.read_u32::<BigEndian>().unwrap();
    let points = cursor.read_u32::<BigEndian>().unwrap();

    ArchiveInfo {
        offset: offset,
        seconds_per_point: seconds_per_point,
        points: points
    }
}
