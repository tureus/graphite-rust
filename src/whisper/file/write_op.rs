use std::io::SeekFrom;

#[derive(PartialEq,Debug)]
pub struct WriteOp {
    pub seek: SeekFrom,
    pub bytes: [u8; 12]
}
