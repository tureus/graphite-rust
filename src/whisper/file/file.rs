use std::path::Path;
use std::io::Error;

use whisper::schema::Schema;
use whisper::point;

pub trait WhisperFile {
    fn open(&Path) -> Result<Self, Error>;
    fn new(path: &Path, schema: Schema /* , _: Metadata */) -> Result<Self, Error>;
    fn write(&mut self, current_time: u64, point: point::Point);
}
