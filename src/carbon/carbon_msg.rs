use std::path::PathBuf;
use std::str;

#[derive(Debug)]
pub struct CarbonError(Error,String);

#[derive(Debug)]
pub enum Error {
    BadDatagram
}

use super::super::whisper::Point;

#[derive(Debug, PartialEq)]
pub struct CarbonMsg {
    pub metric_rel_path: PathBuf,
    pub point: Point
}


impl CarbonMsg {
    pub fn from_datagram(datagram_buffer: &[u8]) -> Result<CarbonMsg, CarbonError> {
        let datagram = match str::from_utf8(datagram_buffer) {
            Ok(body) => body,
            Err(_) => return Err( CarbonError(Error::BadDatagram, "invalid utf8 character".to_string() ))
        };

        // TODO: this seems too complicated just to detect/remove "\n"
        // And it scans the string as utf8 codepoints.
        // Will this just be ASCII... is it safe to skip utf8-ness? (probs not)
        let (without_newline,newline_str) = {
            let len = datagram.len();
            ( 
              datagram.slice_chars(0, len-1),
              datagram.slice_chars(len-1, len)
            )
        };
        if newline_str != "\n" {
            return Err( CarbonError(Error::BadDatagram, format!("Datagram `{}` is missing a newline `{}`", datagram, newline_str)))
        }

        let parts : Vec<&str> = without_newline.split(" ").collect();
        if parts.len() != 3 {
            return Err( CarbonError(Error::BadDatagram, format!("Datagram `{}` does not have 3 parts", datagram) ) );
        }

        // TODO: copies to msg. Used to be a reference from datagram_buffer
        // but figuring out how to keep the datagram_buffer (which was on the heap)
        // alive long enough was tricky.
        let metric_name = parts[0].to_string();
        let mut rel_path : String = metric_name.replace(".","/");
        rel_path.push_str(".wsp");

        let value = {
            let value_parse = parts[1].parse::<f64>();
            match value_parse {
                Ok(val) => val,
                Err(_) => {
                    return Err( CarbonError(Error::BadDatagram, format!("Datagram value `{}` is not a float", parts[1]) ) )
                }
            }
        };

        let timestamp = {
            let timestamp_parse = parts[2].parse::<u64>();
            match timestamp_parse {
                Ok(val) => val,
                Err(_) => {
                    return Err( CarbonError(Error::BadDatagram, format!("Datagram value `{}` is not an unsigned integer", parts[2])) )
                }
            }
        };

        let msg = CarbonMsg {
            metric_rel_path: PathBuf::from(rel_path),
            point: Point {
                value: value,
                timestamp: timestamp                
            }
        };
        Ok(msg)

    }
}
