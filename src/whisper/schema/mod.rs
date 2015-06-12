extern crate regex;

mod retention_policy;

use std::process::exit;

pub use self::retention_policy::RetentionPolicy;

use super::file::{ METADATA_DISK_SIZE, ARCHIVE_INFO_DISK_SIZE };

#[derive(Debug)]
pub struct Schema {
    pub retention_policies: Vec<RetentionPolicy>
}

impl Schema {
    pub fn new_from_retention_specs(specs: Vec<String>) -> Schema {
        let retention_policies : Vec<RetentionPolicy> = {
            let expanded_pairs : Vec<Option<RetentionPolicy>> = specs.iter().map(|ts| {
                parse_spec_to_retention_policy(ts)
            }).collect();

            if expanded_pairs.iter().any(|x| x.is_none()) {
                let specs_iter = specs.iter();
                let pairs_iter = expanded_pairs.iter();
                let error_pairs : Vec<(&String, &Option<RetentionPolicy>)> = specs_iter.zip(pairs_iter).collect();
                validate_retention_policies(&error_pairs);
            }

            expanded_pairs.iter().filter(|x| x.is_some()).map(|x| x.unwrap()).collect()
        };

        Schema{ retention_policies: retention_policies }
    }

    pub fn header_size_on_disk(&self) -> u64 {
        METADATA_DISK_SIZE as u64 +
        (ARCHIVE_INFO_DISK_SIZE*self.retention_policies.len()) as u64
    }

    pub fn size_on_disk(&self) -> u64 {
        let retentions_disk_size = self.retention_policies.iter().fold(0, |tally, policy| {
            debug!("policy: {:?} size on disk: {}", policy, policy.size_on_disk());
            tally + policy.size_on_disk()
        });

        self.header_size_on_disk() + retentions_disk_size
    }

    pub fn max_retention(&self) -> u64 {
        if self.retention_policies.len() == 0 {
            0
        } else {
            self.retention_policies.iter().map(|&rp| rp.retention).max().unwrap()
        }
    }
}


fn mult_str_to_num(mult_str: &str) -> u64 {
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

fn parse_spec_to_retention_policy(spec: &str) -> Option<RetentionPolicy> {
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

fn validate_retention_policies(expanded_pairs: &Vec<(&String, &Option<RetentionPolicy>)> ) {
    let _ : Vec<()> = expanded_pairs.iter().map(|pair: &(&String, &Option<RetentionPolicy>)| {
        let (ref string, ref opt) = *pair;
        if opt.is_none() {
            println!("error: {} is not a valid retention policy", string);
            exit(1);
        }
    }).collect();
}

#[test]
fn test_size_on_disk(){
    let first_policy = RetentionPolicy {
        precision: 1,
        retention: 60
    };

    let second_policy = RetentionPolicy {
        precision: 60,
        retention: 60
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
