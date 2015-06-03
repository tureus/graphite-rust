use whisper::point::POINT_SIZE;

// A RetentionPolicy is the abstract form of an ArchiveInfo
// It does not know it's position in the file
#[derive(Debug, Clone, Copy)]
pub struct RetentionPolicy {
    pub precision: u64,
    pub duration: u64
}

impl RetentionPolicy {
    pub fn size_on_disk(&self) -> u64 {
        // TODO how do we guarantee even divisibility?
        let points = self.duration / self.precision;
        points * POINT_SIZE as u64
    }
}

#[test]
fn test_size_on_disk(){
    let five_minute_retention = RetentionPolicy {
        precision: 60, // 1 sample/minute
        duration: 5*60 // 5 minutes
    };

    let expected = five_minute_retention.size_on_disk();
    assert_eq!(expected, 5*POINT_SIZE as u64);
}
