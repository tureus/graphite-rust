#[derive(PartialEq,Debug)]


pub struct Point {
    pub timestamp: u64,
    pub value: f64
}

// TODO: generate this from the struct definition?
pub const POINT_SIZE : usize = 12;
