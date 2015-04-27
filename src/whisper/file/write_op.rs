use whisper::point::{Point};

#[derive(PartialEq,Debug)]
pub struct WriteOp{
    pub offset: u32,
    pub value: f32
}
