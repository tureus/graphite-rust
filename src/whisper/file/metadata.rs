use std::io::Cursor;
use byteorder::{BigEndian, ReadBytesExt};

#[derive(PartialEq,Debug)]
pub enum AggregationType {
    Average,
    Sum,
    Last,
    Max,
    Min
}

#[derive(PartialEq,Debug)]
pub struct Metadata {
    pub aggregation_type: AggregationType,
    pub max_retention: u32,
    pub x_files_factor: u32,
    pub archive_count: u32
}

pub fn slice_to_metadata(buf: &[u8]) -> Metadata {
    let mut cursor = Cursor::new(buf);

    let aggregation_type = cursor.read_u32::<BigEndian>().unwrap();
    let max_retention = cursor.read_u32::<BigEndian>().unwrap();
    let x_files_factor = cursor.read_u32::<BigEndian>().unwrap();
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
        _ => AggregationType::Average
    }
}
