use std::io::Cursor;
use std::mem::{ size_of };
use byteorder::{ BigEndian, ReadBytesExt, WriteBytesExt, ByteOrder };

use whisper::point::{Point, POINT_SIZE};
use super::write_op::{WriteOp};

#[derive(PartialEq,Debug)]
pub struct ArchiveInfo {
    pub offset: u32,
    pub seconds_per_point: u32,
    pub points: u32,
    pub retention: u32,
    size_in_bytes: usize
}

pub fn slice_to_archive_info(buf: &[u8]) -> ArchiveInfo {
    let mut cursor = Cursor::new(buf);
    let offset = cursor.read_u32::<BigEndian>().unwrap();
    let seconds_per_point = cursor.read_u32::<BigEndian>().unwrap();
    let points = cursor.read_u32::<BigEndian>().unwrap();

    let point_size = POINT_SIZE as u32;
    let size_in_bytes : usize = (seconds_per_point * points * point_size) as usize;

    ArchiveInfo {
        offset: offset,
        seconds_per_point: seconds_per_point,
        points: points,
        retention: seconds_per_point * points,
        size_in_bytes: size_in_bytes
    }
}

impl ArchiveInfo {
    pub fn calculate_offset(&self, point: &Point, base_point: &Point) -> WriteOp {
        if(base_point.timestamp == 0){
            return WriteOp {offset: 0, bytes: [0; 12] };
        } else {

            let file_offset = {
                let time_since_base_time = point.timestamp - base_point.timestamp;
                let points_away_from_base_time = time_since_base_time / self.seconds_per_point;
                let point_size = POINT_SIZE as u32;
                let bytes_away_from_offset = points_away_from_base_time * point_size;
                self.offset + (bytes_away_from_offset % (self.size_in_bytes as u32))
            };

            let output_data = [0; 12];
            {
                let interval_ceiling = point.timestamp - (point.timestamp % self.seconds_per_point);
                let mut cursor = Cursor::new(&mut output_data);
                cursor.write_u32::<BigEndian>(interval_ceiling).unwrap();
                cursor.write_f64::<BigEndian>(point.value);
            }

            return WriteOp {
                offset: file_offset,
                bytes: output_data
            };
        }
    }
}
