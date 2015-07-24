// Just a re-export of the carbon config

use std::path::Path;

pub struct Config<'a> {
    pub bind_spec: &'a str,
    pub base_path: &'a Path
}
