use std::fs::File;

use std::io::{ BufWriter, Cursor, Write };
use byteorder::{ ByteOrder, BigEndian, ReadBytesExt, WriteBytesExt };

// 4 32bit (4-byte) values
pub const METADATA_DISK_SIZE : u64 = 16;

#[derive(PartialEq,Debug)]
pub enum AggregationType {
    Average,
    Sum,
    Last,
    Max,
    Min,
    Unknown
}

#[derive(PartialEq,Debug)]
pub struct Metadata {
    pub aggregation_type: AggregationType,
    pub max_retention: u32,
    pub x_files_factor: f32,
    pub archive_count: u32
}

impl Metadata {
    pub fn write(&self, mut file: &File) {
        let mut arr = [0u8; METADATA_DISK_SIZE as usize];
        let buf : &mut [u8] = &mut arr;

        self.fill_buf(buf);
        file.write_all(buf).unwrap();
    }

    fn fill_buf(&self, buf: &mut [u8]) {
        debug!("writing metadata");
        let mut writer = BufWriter::new(buf);

        let ref agg_val = self.aggregation_type;
        let agg_id = aggregation_type_to_id(&agg_val);
        writer.write_u32::<BigEndian>(agg_id).unwrap();
        writer.write_u32::<BigEndian>(self.max_retention).unwrap();
        writer.write_f32::<BigEndian>(self.x_files_factor).unwrap();
        writer.write_u32::<BigEndian>(self.archive_count).unwrap();
    }
}

pub fn slice_to_metadata(buf: &[u8]) -> Metadata {
    let mut cursor = Cursor::new(buf);

    let aggregation_type = cursor.read_u32::<BigEndian>().unwrap();
    let max_retention = cursor.read_u32::<BigEndian>().unwrap();
    let x_files_factor = cursor.read_f32::<BigEndian>().unwrap();
    let archive_count = cursor.read_u32::<BigEndian>().unwrap();

    Metadata {
        aggregation_type: aggregation_type_from_id(aggregation_type),
        max_retention: max_retention,
        x_files_factor: x_files_factor,
        archive_count: archive_count
    }
}

// TODO change to result type
fn aggregation_type_from_id(id: u32) -> AggregationType{
    match id {
        1 => AggregationType::Average,
        2 => AggregationType::Sum,
        3 => AggregationType::Last,
        4 => AggregationType::Max,
        5 => AggregationType::Min,
        _ => AggregationType::Unknown
    }
}

fn aggregation_type_to_id(aggregation_type: &AggregationType) -> u32 {
    match *aggregation_type {
        AggregationType::Average  => 1,
        AggregationType::Sum  => 2,
        AggregationType::Last  => 3,
        AggregationType::Max  => 4,
        AggregationType::Min  => 5,
        // TODO should be a panic
        AggregationType::Unknown  => 1
    }
}
