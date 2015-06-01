mod retention_policy;

pub use self::retention_policy::RetentionPolicy;

use super::file::{ METADATA_DISK_SIZE, ARCHIVE_INFO_DISK_SIZE };

// #[derive(Debug)]
pub struct Schema {
    pub retention_policies: Vec<RetentionPolicy>
}

impl Schema {
    pub fn size_on_disk(&self) -> u64 {
        let retention_size = self.retention_policies.iter().fold(0, |tally, policy| {
            tally + policy.size_on_disk()
        });

        METADATA_DISK_SIZE as u64 + ARCHIVE_INFO_DISK_SIZE as u64 +retention_size
    }
}

#[test]
fn test_size_on_disk(){
    let first_policy = RetentionPolicy {
        precision: 1,
        points: 60
    };

    let second_policy = RetentionPolicy {
        precision: 60,
        points: 60
    };


    let mut little_schema = Schema {
        retention_policies: vec![]
    };

    let expected = METADATA_DISK_SIZE as u64
            + first_policy.size_on_disk()
            + second_policy.size_on_disk();

    little_schema.retention_policies.push(first_policy);
    little_schema.retention_policies.push(second_policy);

    assert_eq!(little_schema.size_on_disk(), expected);
}
