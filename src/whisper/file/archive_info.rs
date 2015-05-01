use std::io::{ Cursor, BufWriter, SeekFrom  };
use byteorder::{ BigEndian, ReadBytesExt, WriteBytesExt, ByteOrder };

use whisper::point::{Point, POINT_SIZE};
use super::write_op::{WriteOp};

#[derive(PartialEq,Debug)]
pub struct ArchiveInfo {
    pub offset: u64,
    pub seconds_per_point: u64,
    pub points: u64,
    pub retention: u64,
    size_in_bytes: u64
}

pub fn slice_to_archive_info(buf: &[u8]) -> ArchiveInfo {
    let mut cursor = Cursor::new(buf);
    let offset = cursor.read_u32::<BigEndian>().unwrap();
    let seconds_per_point = cursor.read_u32::<BigEndian>().unwrap();
    let points = cursor.read_u32::<BigEndian>().unwrap();

    let point_size = POINT_SIZE as u32;
    let size_in_bytes = (seconds_per_point * points * point_size) as u64;

    ArchiveInfo {
        offset: offset as u64,
        seconds_per_point: seconds_per_point as u64,
        points: points as u64,
        retention: (seconds_per_point * points) as u64,
        size_in_bytes: size_in_bytes
    }
}

impl ArchiveInfo {
    pub fn calculate_seek(&self, point: &Point, base_point: &Point) -> SeekFrom {
        if(base_point.timestamp == 0){
            return SeekFrom::Start(0);
        } else {

            let file_offset = {
                let time_since_base_time = (point.timestamp - base_point.timestamp) as u64;
                let points_away_from_base_time = time_since_base_time / self.seconds_per_point;
                let point_size = POINT_SIZE as u64;
                let bytes_away_from_offset = (points_away_from_base_time * point_size) as u64;
                self.offset + (bytes_away_from_offset % (self.size_in_bytes))
            };

            return SeekFrom::Start(file_offset);

//            let mut output_data = [0; 12];
//            {
//                let interval_ceiling = point.timestamp - (point.timestamp % self.seconds_per_point);
//                let mut buf : &mut [u8] = &mut output_data;
//                let mut writer = BufWriter::new(buf);
//                writer.write_u32::<BigEndian>(interval_ceiling).unwrap();
//                writer.write_f64::<BigEndian>(point.value);
//            }

//            return WriteOp {
//                offset: file_offset,
//                bytes: output_data
//            };
        }
    }
}
