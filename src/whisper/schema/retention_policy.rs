extern crate regex;

use whisper::point::POINT_SIZE;
use whisper::file::ARCHIVE_INFO_DISK_SIZE;

use byteorder::{ ByteOrder, BigEndian, WriteBytesExt };
use std::io::{ BufWriter, Write };
use std::fs::File;

use std::process::exit;

// A RetentionPolicy is the abstract form of an ArchiveInfo
// It does not know it's position in the file. Should it just
// be collapsed in to ArchiveInfo? Possibly.
#[derive(Debug, Clone, Copy)]
pub struct RetentionPolicy {
    pub precision: u64,
    pub retention: u64
}

impl RetentionPolicy {
    pub fn spec_to_retention_policy(spec: &str) -> Option<RetentionPolicy> {
        // TODO: regex should be built as const using macro regex!
        // but that's only available in nightlies.
        let retention_matcher = regex::Regex::new({r"^(\d+)([smhdwy])?:(\d+)([smhdwy])?$"}).unwrap();
        match retention_matcher.captures(spec) {
            Some(regex_match) => {
                retention_capture_to_pair(regex_match)
            },
            None => None
        }
    }

    // TODO how do we guarantee even divisibility?
    pub fn points(&self) -> u64 {
        self.retention / self.precision
    }

    pub fn size_on_disk(&self) -> u64 {
        self.points() * POINT_SIZE as u64
    }

    pub fn write(&self, mut file: &File, offset: u64) {
        debug!("writing retention policy (offset: {})", offset);
        let mut arr = [0u8; ARCHIVE_INFO_DISK_SIZE as usize];
        let buf : &mut [u8] = &mut arr;

        self.fill_buf(buf, offset);
        file.write_all(buf).unwrap();
    }

    pub fn fill_buf(&self, buf: &mut [u8], offset: u64) {
        let mut writer = BufWriter::new(buf);
        let points = self.retention / self.precision;

        writer.write_u32::<BigEndian>(offset as u32).unwrap();
        writer.write_u32::<BigEndian>(self.precision as u32).unwrap();
        writer.write_u32::<BigEndian>(points as u32).unwrap();
    }
}

fn retention_capture_to_pair(regex_match: regex::Captures) -> Option<RetentionPolicy> {
    let precision_opt = regex_match.at(1);
    let precision_mult = regex_match.at(2).unwrap_or("s");
    let retention_opt = regex_match.at(3);
    let retention_mult = regex_match.at(4);

    if precision_opt.is_some() && retention_opt.is_some() {
        let precision = {
            let base_precision = precision_opt.unwrap().parse::<u64>().unwrap();
            base_precision * mult_str_to_num(precision_mult)
        };

        let retention = {
            let base_retention = retention_opt.unwrap().parse::<u64>().unwrap();

            match retention_mult {
                Some(mult_str) => {
                    base_retention * mult_str_to_num(mult_str)
                },
                None => {
                    // user has not provided a multipler so this is interpreted
                    // as the number of points so we have to
                    // calculate retention from the number of points
                    base_retention * precision
                }
            }
        };

        let retention_spec = RetentionPolicy {
            precision: precision,
            retention: retention
        };

        Some(retention_spec)
    } else {
        None
    }
}

fn mult_str_to_num(mult_str: &str) -> u64 {
    // TODO: is this exactly how whisper does it?
    match mult_str {
        "s" => 1,
        "m" => 60,
        "h" => 60*60,
        "d" => 60*60*24,
        "w" => 60*60*24*7,
        "y" => 60*60*24*365,
        _   => {
            // should never pass regex
            println!("All retention policies must be valid. Exiting.");
            exit(1);
        }
    }
}

#[test]
fn test_size_on_disk(){
    let five_minute_retention = RetentionPolicy {
        precision: 60, // 1 sample/minute
        retention: 5*60 // 5 minutes
    };

    let expected = five_minute_retention.size_on_disk();
    assert_eq!(expected, 5*POINT_SIZE as u64);
}

#[test]
fn test_spec_without_multipliers() {
    let spec = "15:60";
    let expected = RetentionPolicy {
        precision: 15,
        retention: 15*60
    };

    let retention_opt = RetentionPolicy::spec_to_retention_policy(spec);
    assert!(retention_opt.is_some());
    let retention_policy = retention_opt.unwrap();
    assert_eq!(retention_policy.precision, expected.precision);
    assert_eq!(retention_policy.retention, expected.retention);
}

#[test]
fn test_spec_with_multipliers() {
    let spec = "1d:60y";
    let expected = RetentionPolicy {
        precision: 1 *  60*60*24,
        retention: 60 * 60*60*24*365
    };
    
    let retention_opt = RetentionPolicy::spec_to_retention_policy(spec);
    assert!(retention_opt.is_some());
    let retention_policy = retention_opt.unwrap();
    assert_eq!(retention_policy.precision, expected.precision);
    assert_eq!(retention_policy.retention, expected.retention);
}
