use std::fs::File;
use std::io::Read;
use whisper;

const HEADER_SIZE: usize = 16;
const ARCHIVE_INFO_SIZE: usize = 12;

#[derive(PartialEq,Debug)]
pub struct Header{
    pub metadata: whisper::metadata::Metadata,
    pub archive_infos: Vec<whisper::archive_info::ArchiveInfo>
}

// TODO change to result type
pub fn read_header(mut file: File) -> Header{
    let header_buffer = &mut[0u8; HEADER_SIZE];
    let metadata_read_result = file.read(header_buffer);

    match metadata_read_result {
        Ok(bytes_read) => {
            if bytes_read != HEADER_SIZE {
                panic!("could not read enough bytes!")
            }

            let metadata = whisper::metadata::slice_to_metadata(header_buffer);

            let mut archive_infos = vec![];

            for archive_index in 0..metadata.archive_count {
            let archive_info_buffer = &mut[0u8; ARCHIVE_INFO_SIZE];
            let archive_read_result = file.read(archive_info_buffer);
            match archive_read_result {
                Ok(bytes_read) => {
                    if bytes_read != ARCHIVE_INFO_SIZE {
                        panic!("could not read enough bytes!")
                    }

                    let archive_info = whisper::archive_info::slice_to_archive_info(archive_info_buffer);
                    archive_infos.push(archive_info);
                }
                Err(_) => {
                    println!("sup");
                }
            }

            println!("archive_index: {}", archive_index);
            }

            let header = Header {
                metadata: metadata,
                archive_infos: archive_infos
            };
            header
        }
        Err(e) => {
            panic!("got err {}", e);
        }
    }
}

#[test]
fn parses_60_1440() {
    let path = "./test/fixtures/60-1440.wsp";
    let open_result = whisper::file::open(path);

    let expected = Header {
        metadata: whisper::metadata::Metadata {
            aggregation_type: whisper::metadata::AggregationType::Average,
            max_retention: 86400,
            x_files_factor: 1056964608,
            archive_count: 1
        },
        archive_infos: vec![
            whisper::archive_info::ArchiveInfo {
                offset: 28,
                seconds_per_point: 60,
                points: 1440
            }
        ]
    };

    match open_result {
        Ok(f) => {
            let header = whisper::header::read_header(f);
            assert_eq!(header, expected)
        }
        Err(_) => {
            assert!(false)
        }
    }
}
