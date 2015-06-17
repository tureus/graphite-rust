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
    pub offset: SeekFrom,
    pub seconds_per_point: u64,
    pub points: u64,
    pub retention: u64,
}

// Index in to an archive, 0..points.len
#[derive(Debug, PartialEq, PartialOrd)]
pub struct ArchiveIndex(pub u64);

// A normalized timestamp
pub struct BucketName(pub u64);

pub fn slice_to_archive_info(buf: &[u8]) -> ArchiveInfo {
    let mut cursor = Cursor::new(buf);
    let offset = cursor.read_u32::<BigEndian>().unwrap();
    let seconds_per_point = cursor.read_u32::<BigEndian>().unwrap();
    let points = cursor.read_u32::<BigEndian>().unwrap();

    ArchiveInfo {
        offset: SeekFrom::Start(offset as u64),
        seconds_per_point: seconds_per_point as u64,
        points: points as u64,
        retention: (seconds_per_point * points) as u64
    }
}

impl ArchiveInfo {
    pub fn size_in_bytes(&self) -> u64 {
        self.points * POINT_SIZE as u64
    }

    pub fn calculate_seek(&self, point: &Point, archive_anchor: BucketName) -> SeekFrom {
        if archive_anchor.0 == 0 {

            return self.offset;

        } else {

            let time_since_base_time = (point.timestamp - archive_anchor.0) as u64;
            let points_away_from_base_time = time_since_base_time / self.seconds_per_point;
            let point_size = POINT_SIZE as u64;
            let bytes_away_from_offset = (points_away_from_base_time * point_size) as u64;

            match self.offset {
                SeekFrom::Start(offset) => {
                    SeekFrom::Start(offset + (bytes_away_from_offset % (self.size_in_bytes())))
                },
                _ => panic!("we only use SeekFrom::Start")
            }

        }
    }

    pub fn bucket(&self, timestamp: u64) -> BucketName {
        let bucket_name = timestamp - (timestamp % self.seconds_per_point);
        BucketName(bucket_name)
    }

    pub fn anchor_bucket(&self, mut file: RefMut<File>) -> BucketName {
        let mut points_buf : [u8; 12] = [0; 12];

        let point = {
            file.seek(self.offset).unwrap();

            let mut buf_ref : &mut [u8] = &mut points_buf;
            file.read(buf_ref).unwrap();

            buf_to_point(buf_ref)
        };

        BucketName(point.timestamp)
    }

    pub fn read_points (&self, archive_index: ArchiveIndex, points: &mut [Point], mut file: RefMut<File>) {
        let index_start = archive_index.0;
        
         // Confirm we aren't ready a contiguous block out of the archive
        assert!( (index_start + points.len() as u64) <= self.points );

        let read_start_offset = match self.offset {
            SeekFrom::Start(offset) => {
                offset + index_start * POINT_SIZE as u64
            },
            _ => panic!("We only use SeekFrom::Start")
        };
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
        offset: SeekFrom::Start(28),
        seconds_per_point: 60,
        retention: 60*5,
        points: 5
    };

    assert_eq!(archive_info.size_in_bytes(), 5*12);
}
