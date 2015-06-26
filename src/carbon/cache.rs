// The cache models the whisper files on the filesystem
// It handles putting all datapoints in to the files

use super::CarbonMsg;
use super::super::whisper::WhisperFile;
use super::super::whisper::schema::Schema;
use std::collections::HashMap;
use std::path::{ Path, PathBuf };
use std::fs::{ PathExt, DirBuilder, metadata };
use std::io;

#[derive(Debug)]
pub struct Cache {
    base_path: PathBuf,
    open_files: HashMap<PathBuf, WhisperFile>
}

impl Cache {
    pub fn new(base_path: &Path) -> Cache {
        Cache {
            base_path: base_path.to_path_buf(),
            open_files: HashMap::new()
        }
    }

    pub fn write(&mut self, current_time: u64, incoming: CarbonMsg) -> Result<(), io::Error> {

        let mut whisper_file = try!( self.resolve(incoming.metric_rel_path) );
        whisper_file.write(current_time, incoming.point);
        Ok(())

    }

    // Find or initialize the whisper file
    fn resolve(&mut self, metric_rel_path: PathBuf) -> Result<&mut WhisperFile, io::Error> {

        if self.open_files.contains_key(&metric_rel_path) {

            debug!("file cache hit. resolved {:?}", metric_rel_path);
            Ok( self.open_files.get_mut(&metric_rel_path).unwrap() )

        } else {

            debug!("file cache miss. resolving {:?}", metric_rel_path);

            let path_for_insert = metric_rel_path.clone();
            let path_for_relookup = metric_rel_path.clone();

            let path_on_disk = self.base_path.join(metric_rel_path);

            let whisper_file = if path_on_disk.exists() && path_on_disk.is_file() {

                debug!("`{:?}` exists on disk. opening.", path_on_disk);
                // TODO: might make sense push this logic to instantiation of CarbonMsg
                // TODO: shouldn't be a UTF8 error cuz of std::str::from_utf8() in CarbonMsg input
                try!( WhisperFile::open(&path_on_disk) )

            } else {

                debug!("`{:?}` file does not exist on disk. creating default.", path_on_disk);
                let default_specs = vec!["1s:60s".to_string(), "1m:1y".to_string()];
                let schema = Schema::new_from_retention_specs(default_specs);

                // Verify the folder structure is present.
                // TODO: benchmark (for my own curiosity)
                // TODO: assumption here is that we do not store in root FS
                if !path_on_disk.parent().unwrap().is_dir() {
                    debug!("`{:?}` must be created first", path_on_disk.parent());
                    try!( DirBuilder::new().recursive(true).create( path_on_disk.parent().unwrap() ) );
                }
                try!( WhisperFile::new(&path_on_disk, schema) )

            };

            self.open_files.insert(path_for_insert, whisper_file);
            Ok( self.open_files.get_mut(&path_for_relookup).unwrap() )

        }
    }
}

#[cfg(test)]
mod test {
    extern crate test;
    use test::Bencher;

    use std::path::{ Path };

    use super::Cache;
    use super::super::CarbonMsg;

    use super::super::super::whisper::Point;

    #[bench]
    fn test_opening_new_whisper_file(b: &mut Bencher){
        let mut cache = Cache::new(Path::new("/tmp"));
        let current_time = 1434598525;

        b.iter(move ||{

            let metric = CarbonMsg {
                metric_rel_path: Path::new("hey/there/bear.wsp").to_path_buf(),
                point: Point {
                    value: 0.0,
                    timestamp: 1434598525
                }
            };

            cache.write(current_time, metric).unwrap();

        });
    }
}
