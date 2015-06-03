mod retention_policy;

use std::fs::File;

pub use self::retention_policy::RetentionPolicy;

use super::file::{ METADATA_DISK_SIZE, ARCHIVE_INFO_DISK_SIZE };

#[derive(Debug)]
pub struct Schema {
    pub retention_policies: Vec<RetentionPolicy>
}

impl Schema {
    pub fn size_on_disk(&self) -> u64 {
        let retentions_disk_size = self.retention_policies.iter().fold(0, |tally, policy| {
            debug!("policy: {:?} size on disk: {}", policy, policy.size_on_disk());
            tally + policy.size_on_disk()
        });

        METADATA_DISK_SIZE as u64 +
        (ARCHIVE_INFO_DISK_SIZE*self.retention_policies.len()) as u64 +
        retentions_disk_size
    }

    pub fn prepare(&self, _: File) {
        println!("sup file!");
    }
}

#[test]
fn test_size_on_disk(){
    let first_policy = RetentionPolicy {
        precision: 1,
        duration: 60
    };

    let second_policy = RetentionPolicy {
        precision: 60,
        duration: 60
    };


    let mut little_schema = Schema {
        retention_policies: vec![]
    };

    let expected = METADATA_DISK_SIZE as u64
            + ARCHIVE_INFO_DISK_SIZE as u64 * 2
            + 60*12 // first policy size
            + 1*12; // second policy size

    little_schema.retention_policies.push(first_policy);
    little_schema.retention_policies.push(second_policy);

    assert_eq!(little_schema.size_on_disk(), expected);
}
