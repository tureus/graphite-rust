use std::fs::File;
use std::io::Read;
use std::io::{ Error, ErrorKind };

#[cfg(test)]
use super::file;

use super::metadata;
use super::archive_info;
use super::archive_info::ARCHIVE_INFO_DISK_SIZE;

pub const HEADER_SIZE: usize = 16;

#[derive(PartialEq,Debug)]
pub struct Header{
    pub metadata: metadata::Metadata,
    pub archive_infos: Vec<archive_info::ArchiveInfo>
}

pub fn read_header(mut file: &File) -> Result<Header, Error> {
    let header_buffer = &mut[0u8; HEADER_SIZE];
    let bytes_read = try!(file.read(header_buffer));
    if bytes_read != HEADER_SIZE {
        return Err(Error::new(ErrorKind::Other, "could not read enough bytes for metadata"));
    }

    let metadata = metadata::slice_to_metadata(header_buffer);
    let archive_infos : Vec<archive_info::ArchiveInfo> = (0..metadata.archive_count).map(|_| {
        let archive_info_buffer = &mut[0u8; ARCHIVE_INFO_DISK_SIZE];
        let archive_bytes_read = try!(file.read(archive_info_buffer));

        // TODO: this return is for the block. I want it for the function.
        if archive_bytes_read != ARCHIVE_INFO_DISK_SIZE {
            return Err(Error::new(ErrorKind::Other, "could not get enough bytes for index"));
        }

        Ok(archive_info::slice_to_archive_info(archive_info_buffer))
    }).filter(|m| m.is_ok()).map(|m| m.unwrap()).collect();

    let header = Header {
        metadata: metadata,
        archive_infos: archive_infos
    };
    Ok(header)
}

#[test]
fn parses_60_1440() {
    use std::io::SeekFrom;
    
    let path = "./test/fixtures/60-1440.wsp";
    let f = file::open(path).unwrap();

    // A literal Header
    let expected = Header {
        metadata: metadata::Metadata {
            aggregation_type: metadata::AggregationType::Average,
            max_retention: 86400,
            x_files_factor: 0.5,
            archive_count: 1
        },
        archive_infos: vec![
            archive_info::ArchiveInfo {
                offset: SeekFrom::Start(28),
                seconds_per_point: 60,
                points: 1440,
                retention: 60*1440
            }
        ]
    };

    assert_eq!(f.header, expected)
}
