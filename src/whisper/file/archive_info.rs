use std::io::{ Cursor, SeekFrom  };
use std::fmt;
use byteorder::{ BigEndian, ReadBytesExt, ByteOrder };

use whisper::point::{Point, POINT_SIZE};

// offset (32bit) + seconds per point (32bit) + number of points (32bit)
pub const ARCHIVE_INFO_DISK_SIZE : usize = 12;

//Don't think we need Copy/Clone. Just added it to make tests easier to write.
#[derive(PartialEq,Copy,Clone)]
pub struct ArchiveInfo {
    pub offset: u64,
    pub seconds_per_point: u64,
    pub points: u64,
    pub retention: u64,

    // TODO: made public so I can use it in tests
    pub size_in_bytes: u64
}

impl fmt::Debug for ArchiveInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,
"       Archive
            offset: {}
            seconds per point: {}
            points: {}
            retention: {}

",
        self.offset,
        self.seconds_per_point,
        self.points,
        self.retention
)
    }
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
    pub fn calculate_seek(&self, point: &Point, base_timestamp: u64) -> SeekFrom {
        if base_timestamp == 0 {
            return SeekFrom::Start(self.offset);
        } else {

            let file_offset = {
                let time_since_base_time = (point.timestamp - base_timestamp) as u64;
                let points_away_from_base_time = time_since_base_time / self.seconds_per_point;
                let point_size = POINT_SIZE as u64;
                let bytes_away_from_offset = (points_away_from_base_time * point_size) as u64;
                self.offset + (bytes_away_from_offset % (self.size_in_bytes))
            };

            return SeekFrom::Start(file_offset);
        }
    }

    pub fn interval_ceiling(&self, timestamp: u64) -> u64 {
        let retval = timestamp - (timestamp % self.seconds_per_point);
        // debug!("timestamp: {}, interval: {}", timestamp, retval);
        retval
    }
}
