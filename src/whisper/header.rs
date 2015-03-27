use std::fs::File;
use std::io::Read;
use whisper::metadata;
use whisper::archive_info;

const HEADER_SIZE: usize = 16;
const ARCHIVE_INFO_SIZE: usize = 12;

#[derive(Debug)]
pub struct Header{
  pub metadata: metadata::Metadata,
  pub archive_infos: Vec<archive_info::ArchiveInfo>
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

      let metadata = metadata::slice_to_metadata(header_buffer);

      // let archive_sequence = 0..metadata.archive_count;
      let mut archive_infos = vec![];
      for archive_index in 0..metadata.archive_count {
        let archive_info_buffer = &mut[0u8; ARCHIVE_INFO_SIZE];
        let archive_read_result = file.read(archive_info_buffer);
        match archive_read_result {
          Ok(bytes_read) => {
            let archive_info = archive_info::slice_to_archive_info(archive_info_buffer);
            println!("archive_info: {:?}", archive_info);
            archive_infos.push(archive_info);
          }
          Err(_) => {
            println!("sup");
          }
        }

        println!("archive_index: {}", archive_index);
        // archive_infos[archive_index] = 
      }

      Header {
        metadata: metadata,
        archive_infos: archive_infos
      }
    }
    Err(e) => {
      panic!("got err {}", e);
    }
  }
}