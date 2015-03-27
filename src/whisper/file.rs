use std::io::Error;
use std::fs::File;
use std::path::Path;

pub fn allocate(path:& str) -> Result<File, Error>{
  return File::create(Path::new(path));
}

pub fn open(path:& str) -> Result<File, Error> {
  return File::open(Path::new(path));
}