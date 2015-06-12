use std::io::{ Cursor, SeekFrom, Seek, Read  };
use byteorder::{ BigEndian, ReadBytesExt, ByteOrder };
use std::fs::File;
use std::cell::RefMut;

use whisper::point::{Point, POINT_SIZE, buf_to_point};

// offset (32bit) + seconds per point (32bit) + number of points (32bit)
pub const ARCHIVE_INFO_DISK_SIZE : usize = 12;

// TODO: Don't think we need Copy/Clone. Just added it to make tests easier to write.
#[derive(PartialEq,Copy,Clone,Debug)]
pub struct ArchiveInfo {
    pub offset: u64,
    pub seconds_per_point: u64,
    pub points: u64,
    pub retention: u64,
}


pub fn slice_to_archive_info(buf: &[u8]) -> ArchiveInfo {
    let mut cursor = Cursor::new(buf);
    let offset = cursor.read_u32::<BigEndian>().unwrap();
    let seconds_per_point = cursor.read_u32::<BigEndian>().unwrap();
    let points = cursor.read_u32::<BigEndian>().unwrap();

    ArchiveInfo {
        offset: offset as u64,
        seconds_per_point: seconds_per_point as u64,
        points: points as u64,
        retention: (seconds_per_point * points) as u64
    }
}

impl ArchiveInfo {
    pub fn size_in_bytes(&self) -> u64 {
        self.points * POINT_SIZE as u64
    }

    pub fn calculate_seek(&self, point: &Point, base_timestamp: u64) -> SeekFrom {
        if base_timestamp == 0 {
            return SeekFrom::Start(self.offset);
        } else {

            let file_offset = {
                let time_since_base_time = (point.timestamp - base_timestamp) as u64;
                let points_away_from_base_time = time_since_base_time / self.seconds_per_point;
                let point_size = POINT_SIZE as u64;
                let bytes_away_from_offset = (points_away_from_base_time * point_size) as u64;
                self.offset + (bytes_away_from_offset % (self.size_in_bytes()))
            };

            return SeekFrom::Start(file_offset);
        }
    }

    pub fn interval_ceiling(&self, timestamp: u64) -> u64 {
        timestamp - (timestamp % self.seconds_per_point)
    }

    pub fn read_points (&self, index_start: u64, points: &mut [Point], mut file: RefMut<File>) {
        let points_len = points.len() as u64;
        let read_to = index_start + points_len;
        if (index_start + points.len() as u64) > self.points {
            panic!("self.points: {}, points.len(): {}", self.points, points.len());
            panic!("index_start: {}, points: {}, read_to: {}", index_start, points.len(), read_to);
        }
        // Confirm we aren't ready a contiguous block out of the archive
        assert!( (index_start + points.len() as u64) <= self.points );

        let read_start_offset = self.offset + index_start * POINT_SIZE as u64;
        let mut points_buf = vec![0; points.len() * POINT_SIZE];

        file.seek( SeekFrom::Start(read_start_offset) ).unwrap();
        let bytes_read = file.read(&mut points_buf[..]).unwrap();
        assert_eq!(bytes_read, points_buf.len());

        let buf_chunks = points_buf.chunks(POINT_SIZE);
        let index_chunk_pairs = (0..points.len()).zip(buf_chunks);

        for (index,chunk) in index_chunk_pairs {
            points[index] = buf_to_point(chunk);
        }
    }

}

#[test]
fn test_size_in_bytes(){
    let archive_info = ArchiveInfo {
        offset: 28,
        seconds_per_point: 60,
        retention: 60*5,
        points: 5
    };

    assert_eq!(archive_info.size_in_bytes(), 5*12);
}
