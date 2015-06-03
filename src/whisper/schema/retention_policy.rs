use whisper::point::POINT_SIZE;
use whisper::file::ARCHIVE_INFO_DISK_SIZE;

use byteorder::{ ByteOrder, BigEndian, WriteBytesExt };
use std::io::{ BufWriter, Write };
use std::fs::File;

// A RetentionPolicy is the abstract form of an ArchiveInfo
// It does not know it's position in the file
#[derive(Debug, Clone, Copy)]
pub struct RetentionPolicy {
    pub precision: u64,
    pub retention: u64
}

impl RetentionPolicy {
    pub fn size_on_disk(&self) -> u64 {
        // TODO how do we guarantee even divisibility?
        let points = self.retention / self.precision;
        points * POINT_SIZE as u64
    }

    pub fn write(&self, mut file: &File, offset: u64) {
        debug!("writing retention policy (offset: {})", offset);
        let mut arr = [0u8; ARCHIVE_INFO_DISK_SIZE as usize];
        let buf : &mut [u8] = &mut arr;

        self.fill_buf(buf, offset);
        file.write_all(buf).unwrap();
    }

    pub fn fill_buf(&self, buf: &mut [u8], offset: u64) {
        let mut writer = BufWriter::new(buf);
        let points = self.retention / self.precision;

        writer.write_u32::<BigEndian>(offset as u32).unwrap();
        writer.write_u32::<BigEndian>(self.precision as u32).unwrap();
        writer.write_u32::<BigEndian>(points as u32).unwrap();
    }
}

#[test]
fn test_size_on_disk(){
    let five_minute_retention = RetentionPolicy {
        precision: 60, // 1 sample/minute
        retention: 5*60 // 5 minutes
    };

    let expected = five_minute_retention.size_on_disk();
    assert_eq!(expected, 5*POINT_SIZE as u64);
}
