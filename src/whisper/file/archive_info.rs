use std::io::Cursor;
use std::mem::{ size_of };
use byteorder::{ BigEndian, ReadBytesExt, WriteBytesExt, ByteOrder };

use whisper::point::{Point};
use super::write_op::{WriteOp};

#[derive(PartialEq,Debug)]
pub struct ArchiveInfo {
    pub offset: u32,
    pub seconds_per_point: u32,
    pub points: u32,
    pub retention: u32,
    pub base_point: Point
}

pub fn slice_to_archive_info(buf: &[u8]) -> ArchiveInfo {
    let mut cursor = Cursor::new(buf);
    let offset = cursor.read_u32::<BigEndian>().unwrap();
    let seconds_per_point = cursor.read_u32::<BigEndian>().unwrap();
    let points = cursor.read_u32::<BigEndian>().unwrap();

    ArchiveInfo {
        offset: offset,
        seconds_per_point: seconds_per_point,
        points: points,
        retention: seconds_per_point * points
    }
}

impl ArchiveInfo {
    pub fn calculate_offset(&self, point: &Point, base_point: &Point) -> WriteOp {
        let interval_ceiling = point.timestamp - (point.timestamp % self.seconds_per_point);

        let point_size = size_of(Point);

        let file_offset = self.offset + 

        let output_data = [0; 12];
        Cursor::new(output_data);
        cursor.write_u32::<BigEndian>(point.timestamp);
        cursor.write_f64::<BigEndian>(point.value);

        return WriteOp {
            offset: self.offset,
            bytes: file_offset
        };
    }
}
