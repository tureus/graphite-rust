#[derive(PartialEq,Debug)]
pub struct WriteOp {
    pub offset: u32,
    pub bytes: [u8; 12]
}
