use std::mem::{size_of};

#[derive(PartialEq,Debug)]
pub struct Point {
    pub timestamp: u32,
    pub value: f64
}

const POINT_SIZE : usize = size_of::<Point>();
