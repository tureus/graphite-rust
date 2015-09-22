use std::path::Path;

pub struct Config<'a> {
    pub bind_spec: &'a str,
    pub chan_depth: usize,
    pub base_path: &'a Path,
    pub cache_size: usize
}
