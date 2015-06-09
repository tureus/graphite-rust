use std::fs::File;
use std::io::{SeekFrom, Seek, Read, Write};
use std::fs::OpenOptions;
use std::fmt;
use num::iter::range_step_inclusive;
use std::cell::RefCell;
use std::io::Error;

extern crate libc;
use self::libc::funcs::posix01::unistd::ftruncate;
use std::os::unix::prelude::AsRawFd;

use super::header::{ Header, read_header };
use super::write_op::WriteOp;
use super::archive_info::ArchiveInfo;
use super::metadata::{Metadata, AggregationType};
use whisper::schema::Schema;

use whisper::point;

pub struct WhisperFile<'a> {
    pub path: &'a str,
    pub handle: RefCell<File>,
    pub header: Header
}

impl<'a> fmt::Debug for WhisperFile<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "path:  {:?}, header: {:?}", self.path, self.header)
    }
}

impl<'a> fmt::Display for WhisperFile<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ref metadata = self.header.metadata;
        let ref archive_infos = self.header.archive_infos;

        try!(writeln!(f, "whisper file ({})", self.path));
        try!(writeln!(f, "  metadata"));
        try!(writeln!(f, "    aggregation method: {:?}", metadata.aggregation_type));
        try!(writeln!(f, "    max retention: {:?}", metadata.max_retention));
        try!(writeln!(f, "    xff: {:?}", metadata.x_files_factor));

        let mut index = 0;
        for archive_info in archive_infos.iter() {
            // Archive details
            try!(writeln!(f, "  archive {}", index));
            try!(writeln!(f, "    seconds per point: {}", archive_info.seconds_per_point));
            try!(writeln!(f, "    points: {}", archive_info.points));
            try!(writeln!(f, "    retention: {} (s)", archive_info.retention));
            try!(write!(f, "    size: {} (bytes)\n", archive_info.size_in_bytes));

            // Print out all the data from this archive
            // let mut points : Vec<point::Point> = Vec::with_capacity(archive_info.points as usize);
            try!(writeln!(f, "    data"));
            for point_index in (0..archive_info.points) {
                let offset = archive_info.offset + point_index*point::POINT_SIZE as u64;
                let point = self.read_point(offset);
                try!(writeln!(f, "      timestamp {} value {}", point.timestamp, point.value));
            }

            if index != archive_infos.len() - 1 {
                try!(writeln!(f, ""));
            }
            index = index+1;
        }
        write!(f,"") // make the types happy
    }
}

pub fn open(path: &str) -> Result<WhisperFile, Error> {
    let file = try!(OpenOptions::new().read(true).write(true).create(false).open(path));
    let header = try!(read_header(&file));
    let whisper_file = WhisperFile { path: path, header: header, handle: RefCell::new(file) };
    Ok(whisper_file)
}

impl<'a> WhisperFile<'a> {

    pub fn new(path: &str, schema: Schema /* , _: Metadata */) -> Result<WhisperFile, Error> {
        let size_needed = schema.size_on_disk();
        let opened_file = try!(OpenOptions::new().read(true).write(true).create(true).open(path));

        // Allocate the room necessary
        debug!("allocating...");
        {
            let raw_fd = opened_file.as_raw_fd();
            let retval = unsafe {
                // TODO skip to fallocate-like behavior. Will need wrapper for OSX.
                ftruncate(raw_fd, size_needed as i64)
            };
            if retval != 0 {
                return Err(Error::last_os_error());
            }
        }
        debug!("done allocating");

        let metadata = {
            // TODO make agg_t, max_r options from the command line.
            let aggregation_type = AggregationType::Average;
            let x_files_factor = 0.5;
            Metadata {
                aggregation_type: aggregation_type,
                max_retention: schema.max_retention() as u32,
                x_files_factor: x_files_factor,
                archive_count: schema.retention_policies.len() as u32
            }
        };

        // Piggy back on moving file write forward
        metadata.write(&opened_file);

        let initial_archive_offset = schema.header_size_on_disk();
        schema.retention_policies.iter().fold(initial_archive_offset, |offset, &rp| {
            debug!("sup");
            rp.write(&opened_file, offset);
            offset + rp.size_on_disk()
        });

        let new_whisper_file = WhisperFile {
            path: path,
            handle: RefCell::new(opened_file),
            header: Header {
                metadata: metadata,
                archive_infos: vec![]
            }
        };
        Ok(new_whisper_file)
    }

    // TODO: Result<usize> return how many write ops were done
    pub fn write(&mut self, current_time: u64, point: point::Point) {

        match self.split(current_time, point.timestamp) {
            Some( (high_precision_archive, rest) ) => {
                let base_point = self.read_point(high_precision_archive.offset);
                let base_timestamp = base_point.timestamp;

                self.write_archives(
                    (high_precision_archive, rest),
                    point,
                    base_timestamp
                );
            },
            None => {
                panic!("no archives satisfy current time")
            }
        }

    }

    fn perform_write_op(&self, write_op: &WriteOp) {
        let mut handle = self.handle.borrow_mut();
        handle.seek(write_op.seek).unwrap();
        handle.write_all(&(write_op.bytes)).unwrap();
    }

    fn read_point(&self, offset: u64) -> point::Point {
        let mut file = self.handle.borrow_mut();
        file.seek(SeekFrom::Start(offset)).unwrap();

        let mut points_buf : [u8; 12] = [0; 12];
        let mut buf_ref : &mut [u8] = &mut points_buf;
        file.read(buf_ref).unwrap();

        point::buf_to_point(buf_ref)
    }

    pub fn write_archives(&self, (ai,rest): (&ArchiveInfo, Vec<&ArchiveInfo>), point: point::Point, base_timestamp: u64) {
        {
            let write_op = build_write_op( ai, &point, base_timestamp );
            self.perform_write_op(&write_op);
        }

        if rest.len() > 1 {
            self.downsample(ai, rest[0], point.timestamp).map(|write_op| self.perform_write_op(&write_op) );

            let high_res_iter = rest[0..rest.len()-1].into_iter();
            let low_res_iter = rest[1..].into_iter();
            let _ : Vec<()> = high_res_iter.
                zip(low_res_iter).
                take_while(|&(h,l)| {
                    match self.downsample(h, l, point.timestamp) {
                        Some(write_op) => {
                            self.perform_write_op(&write_op);
                            true
                        },
                        None => false
                    }
                }).
                map(|_| ()).
                collect();
        }
    }

    // The most expensive IO functionality
    // Reads many samples from high-res archive and
    // aggregates to lower-res archive. Schemas could do well to avoid
    // aggregation unless disk space is truly at a premium.
    //
    // A cache for each archive would do well here. `memmap` would be awesomesauce.
    fn downsample(&self, h_res_archive: &ArchiveInfo, l_res_archive: &ArchiveInfo, base_timestamp: u64) -> Option<WriteOp> {
        assert!(h_res_archive.seconds_per_point < l_res_archive.seconds_per_point);

        let l_interval_start = l_res_archive.interval_ceiling(base_timestamp);
        // debug!("base_timestamp: {}, l_interval_start: {}", base_timestamp, l_interval_start);

        let h_base_timestamp = self.read_point(h_res_archive.offset).timestamp;
        let h_res_start_offset : u64 = if h_base_timestamp == 0 {
            h_res_archive.offset
        } else {
            // TODO: this can be negative. Does that change timestamp understanding?
            let timespan  = l_interval_start as i64 - h_base_timestamp as i64;
            // debug!("timespan {}", timespan);
            let points = timespan / h_res_archive.seconds_per_point as i64;
            // debug!("points {}", points);
            let bytes = points * point::POINT_SIZE as i64;

            // TODO: OMG, move this craziness somewhere else
            let wrapped_index = {
                let remainder = bytes % h_res_archive.size_in_bytes as i64;
                if remainder < 0 {
                    h_res_archive.size_in_bytes as i64 + remainder
                } else {
                    remainder
                }
            };
            // debug!("offset: {}, wrapped_index: {}, bytes: {}, points: {}", h_res_archive.offset, wrapped_index, bytes, points);
            (h_res_archive.offset as i64 + wrapped_index) as u64
        };

        let h_res_points_needed = l_res_archive.seconds_per_point / h_res_archive.seconds_per_point;
        let h_res_bytes_needed = h_res_points_needed * point::POINT_SIZE as u64;

        let h_res_end_offset = {
            let rel_first_offset = h_res_start_offset - h_res_archive.offset;
            let rel_second_offset = (rel_first_offset + h_res_bytes_needed) % h_res_archive.size_in_bytes;
            h_res_archive.offset + rel_second_offset
        };

        let mut h_res_read_buf : Vec<u8> = Vec::with_capacity(h_res_bytes_needed as usize);

        // Subroutine for filling in the buffer
        {
            let mut handle = self.handle.borrow_mut();

            // TODO: must be a better way of zeroing out a buffer to fill in the vector
            for _ in 0..h_res_bytes_needed {
                h_res_read_buf.push(0);
            }

            // TODO: refactor in to function which
            // returns ((Seek,BytesRead),Option<(Seek,BytesRead)>)
            // so this code can be refactored and unit tested...
            if h_res_start_offset < h_res_end_offset {
                // No wrap situation
                let seek = SeekFrom::Start(h_res_start_offset);

                let mut read_buf : &mut [u8] = &mut h_res_read_buf[..];
                handle.seek(seek).unwrap();
                // debug!("READ FROM {} to {} (contiguous read)", h_res_start_offset, h_res_start_offset + read_buf.len() as u64);
                handle.read(read_buf).unwrap();
            } else {
                let high_res_abs_end = h_res_archive.offset + h_res_archive.size_in_bytes;
                let first_seek = SeekFrom::Start(h_res_start_offset);
                let first_seek_bytes = high_res_abs_end - h_res_start_offset;

                // debug!("READ FROM {} to {} (wrap-around read 1)", h_res_start_offset, h_res_start_offset+first_seek_bytes);

                let (first_buf, second_buf) = h_res_read_buf.split_at_mut(first_seek_bytes as usize);

                handle.seek(first_seek).unwrap();
                handle.read(first_buf).unwrap();

                let second_seek = SeekFrom::Start(h_res_archive.offset);
                // debug!("READ FROM {} to {} (wrap-around read 2)", h_res_archive.offset, h_res_archive.offset + second_buf.len() as u64);

                handle.seek(second_seek).unwrap();
                handle.read(second_buf).unwrap();
            }

        }

        let low_res_aggregate = {
            let points : Vec<point::Point> = h_res_read_buf.chunks(point::POINT_SIZE).map(|chunk| {
                point::buf_to_point(chunk)

            }).collect();

            let timestamp_start = l_interval_start;
            let timestamp_stop = l_interval_start + (h_res_points_needed as u64)*h_res_archive.seconds_per_point;
            let step = h_res_archive.seconds_per_point;

            let expected_timestamps = range_step_inclusive(timestamp_start, timestamp_stop, step);
            let valid_points : Vec<&point::Point> = expected_timestamps.
                zip(points.iter()).
                map(|(ts, p)| {
                    if p.timestamp == ts {
                        // debug!("comparing {} and {}/{} (MATCH!)", ts, p.timestamp, p.value);
                        Some(p)
                    } else {
                        // debug!("comparing {} and {}/{}", ts, p.timestamp, p.value);
                        None
                    }
                }).filter(|agg| !agg.is_none()).map(|agg| agg.unwrap()).collect();
            self.aggregate_samples(valid_points, h_res_points_needed)
        };

        low_res_aggregate.map(|aggregate| {
            let l_res_base_point = self.read_point(l_res_archive.offset);
            let l_res_point = point::Point{ timestamp: l_interval_start, value: aggregate };
            build_write_op(l_res_archive, &l_res_point, l_res_base_point.timestamp)
        })
    }

    fn aggregate_samples(&self, points: Vec<&point::Point>, points_possible: u64) -> Option<f64>{
        // debug!("points: {:?}", points);
        let valid_points : Vec<&&point::Point> = points.iter().filter(|p| p.timestamp != 0).map(|p| p).collect();

        let ratio : f32 = valid_points.len() as f32 / points_possible as f32;
        // debug!("valid_points: {}, len: {}, ratio {} vs xff {}", valid_points.len(), points.len(), ratio, self.header.metadata.x_files_factor);
        if ratio < self.header.metadata.x_files_factor {
            // debug!("not enough data to propagate");
            return None;
        }

        // TODO: we only do aggregation right now!
        match self.header.metadata.aggregation_type {
            AggregationType::Average => {
                let sum = points.iter().map(|p| p.value).fold(0.0, |l, r| l + r);
                Some(sum / points.len() as f64)
            },
            _ => { Some(0.0) }
        }
    }

    fn split(&self, current_time: u64, point_timestamp: u64) -> Option<(&ArchiveInfo, Vec<&ArchiveInfo>)>  {
        let mut archive_iter = self.header.archive_infos.iter();
        
        let high_precision_archive_option = archive_iter.find(|ai|
            ai.retention > (current_time - point_timestamp)
        );

        match high_precision_archive_option {
            Some(ai) => {
                let rest_of_archives = archive_iter;
                let low_res_archives : Vec<&ArchiveInfo> = rest_of_archives.collect();

                Some((ai, low_res_archives))
            },
            None => {
                None
            }
        }
    }
}

fn build_write_op(archive_info: &ArchiveInfo, point: &point::Point, base_timestamp: u64) -> WriteOp {
    let mut output_data = [0; 12];
    let interval_ceiling = archive_info.interval_ceiling(point.timestamp);
    {
        let point_value = point.value;
        let buf : &mut [u8] = &mut output_data;
        point::fill_buf(buf, interval_ceiling, point_value);
    }

    let seek_info = archive_info.calculate_seek(&point,  base_timestamp);

    return WriteOp {
        seek: seek_info,
        bytes: output_data
    }
}

#[cfg(test)]
mod tests {
    use test::Bencher;

    use super::super::archive_info::ArchiveInfo;
    use whisper::point::Point;
    use super::build_write_op;

    #[bench]
    fn bench_build_write_op(b: &mut Bencher) {
        let archive_info = ArchiveInfo {
            offset: 28,
            seconds_per_point: 60,
            points: 1000,
            retention: 10000,
            size_in_bytes: 10000
        };
        let point = Point {
            timestamp: 1000,
            value: 10.0
        };
        let base_timestamp = 900;

        b.iter(|| build_write_op(&archive_info, &point, base_timestamp));
    }
}
