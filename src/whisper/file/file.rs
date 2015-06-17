use std::fs::File;
use std::io::{SeekFrom, Seek, Read, Write};
use std::fs::OpenOptions;
use std::fmt;
use num::iter::{ range_step_inclusive, RangeStepInclusive };
use std::cell::RefCell;
use std::io::Error;

extern crate libc;
use self::libc::funcs::posix01::unistd::ftruncate;
use std::os::unix::prelude::AsRawFd;

use super::header::{ Header, read_header };
use super::write_op::WriteOp;
use super::archive_info::{ ArchiveInfo, ArchiveIndex, BucketName };
use super::metadata::{Metadata, AggregationType};
use whisper::schema::Schema;

use whisper::point;

pub struct WhisperFile {
    pub handle: RefCell<File>,
    pub header: Header
}

impl fmt::Debug for WhisperFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ref metadata = self.header.metadata;
        let ref archive_infos = self.header.archive_infos;

        try!(writeln!(f, "whisper file"));
        try!(writeln!(f, "  metadata"));
        try!(writeln!(f, "    aggregation method: {:?}", metadata.aggregation_type));
        try!(writeln!(f, "    max retention: {:?}", metadata.max_retention));
        try!(writeln!(f, "    xff: {:?}", metadata.x_files_factor));

        for (index,archive_info) in (0..).zip(archive_infos.iter()) {
            // Archive details
            try!(writeln!(f, "  archive {}", index));

            match archive_info.offset {
                SeekFrom::Start(offset) => {
                    try!(writeln!(f, "    offset: {}", offset));
                },
                _ => panic!("We only use SeekFrom::Start")
            }
            try!(writeln!(f, "    seconds per point: {}", archive_info.seconds_per_point));
            try!(writeln!(f, "    points: {}", archive_info.points));
            try!(writeln!(f, "    retention: {} (s)", archive_info.retention));
            try!(write!(f, "    size: {} (bytes)\n", archive_info.size_in_bytes()));

            // Print out all the data from this archive
            try!(writeln!(f, "    data"));

            let mut points : Vec<point::Point> = vec![point::Point{timestamp: 0, value: 0.0}; archive_info.points as usize];
            self.read_points(archive_info.offset, &mut points[..]);
            for point in points {
                try!(writeln!(f, "      timestamp: {} value: {}", point.timestamp, point.value));
            }

            if index != archive_infos.len() - 1 {
                try!(writeln!(f, ""));
            }
        }
        write!(f,"") // make the types happy
    }
}

impl fmt::Display for WhisperFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ref metadata = self.header.metadata;
        let ref archive_infos = self.header.archive_infos;

        try!(writeln!(f, "whisper file"));
        try!(writeln!(f, "  metadata"));
        try!(writeln!(f, "    aggregation method: {:?}", metadata.aggregation_type));
        try!(writeln!(f, "    max retention: {:?}", metadata.max_retention));
        try!(writeln!(f, "    xff: {:?}", metadata.x_files_factor));

        for (index,archive_info) in (0..).zip(archive_infos.iter()) {
            try!(writeln!(f, "  archive {}", index));

            match archive_info.offset {
                SeekFrom::Start(offset) => {
                    try!(writeln!(f, "    offset: {}", offset));
                },
                _ => panic!("We only use SeekFrom::Start")
            }

            try!(writeln!(f, "    seconds per point: {}", archive_info.seconds_per_point));
            try!(writeln!(f, "    points: {}", archive_info.points));
            try!(writeln!(f, "    retention: {} (s)", archive_info.retention));
            try!(write!(f, "    size: {} (bytes)\n", archive_info.size_in_bytes()));

            if index != archive_infos.len() - 1 {
                try!(writeln!(f, ""));
            }
        }
        write!(f,"") // make the types happy
    }
}

// TODO move to impl
pub fn open(path: &str) -> Result<WhisperFile, Error> {
    let file = try!(OpenOptions::new().read(true).write(true)
                        .create(false).open(path));

    let header = try!(read_header(&file));
    let whisper_file = WhisperFile { header: header, handle: RefCell::new(file) };

    Ok(whisper_file)
}

impl WhisperFile {
    pub fn new(path: &str, schema: Schema /* , _: Metadata */) -> Result<WhisperFile, Error> {
        let opened_file = try!(OpenOptions::new().read(true).write(true).create(true).open(path));
        WhisperFile::write_data_layout(opened_file, schema)
    }

    fn write_data_layout(opened_file: File, schema: Schema /* , _: Metadata */) -> Result<WhisperFile, Error> {
        let size_needed = schema.size_on_disk();

        // Allocate the room necessary
        debug!("allocating {} bytes...", size_needed);
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

        let mut archive_offset = schema.header_size_on_disk();

        // write the archive info to disk and build ArchiveInfos
        let archive_infos : Vec<ArchiveInfo> = schema.retention_policies.iter().map(|&rp| {
            rp.write(&opened_file, archive_offset);
            let archive_info = ArchiveInfo {
                offset: SeekFrom::Start(archive_offset),
                seconds_per_point: rp.precision,
                retention: rp.retention,
                points: rp.points()
            };
            archive_offset = archive_offset + rp.size_on_disk();
            archive_info
        }).collect();

        let new_whisper_file = WhisperFile {
            handle: RefCell::new(opened_file),
            header: Header {
                metadata: metadata,
                archive_infos: archive_infos
            }
        };
        Ok(new_whisper_file)
    }

    // Would love to get better stats. How many page misses, how many archive reads, how many samples.
    pub fn write(&mut self, current_time: u64, point: point::Point) {

        let search_opt = self.find_highest_res_archive(current_time, point.timestamp);
        let search_result = search_opt.expect("no archives satisfy current time");
        let (high_precision_archive, rest) = search_result;

        self.write_through_all_archives(
            (high_precision_archive, rest),
            point
        );

    }

    fn perform_write_op(&self, write_op: &WriteOp) {
        let mut handle = self.handle.borrow_mut();
        handle.seek(write_op.seek).unwrap();
        handle.write_all(&(write_op.bytes)).unwrap();
    }

    fn read_point(&self, offset: SeekFrom) -> point::Point {
        let mut file = self.handle.borrow_mut();
        file.seek(offset).unwrap();

        let mut points_buf : [u8; 12] = [0; 12];
        let mut buf_ref : &mut [u8] = &mut points_buf;
        file.read(buf_ref).unwrap();

        point::buf_to_point(buf_ref)
    }

    // Attempt at a weird API: you pass me a slice and I fill it with points.
    fn read_points(&self, offset: SeekFrom, points: &mut [point::Point]) {
        let mut points_buf = vec![0; points.len() * point::POINT_SIZE];

        let mut file = self.handle.borrow_mut();
        file.seek(offset).unwrap();
        let bytes_read = file.read(&mut points_buf[..]).unwrap();
        assert_eq!(bytes_read, points_buf.len());

        let buf_chunks = points_buf.chunks(point::POINT_SIZE);
        let index_chunk_pairs = (0..points.len()).zip(buf_chunks);

        for (index,chunk) in index_chunk_pairs {
            points[index] = point::buf_to_point(chunk);
        }
    }

    fn write_through_all_archives(&self,
                      (h_res,rest): (&ArchiveInfo, Vec<&ArchiveInfo>),
                      point: point::Point) {

        // Write in to the base archive
        {
            let file_handle = self.handle.borrow_mut();
            let h_res_base_bucket = h_res.anchor_bucket(file_handle);
            let write_op = build_write_op( h_res, &point, h_res_base_bucket );
            self.perform_write_op(&write_op);
        }

        // We're done unless there are low-res archives
        if rest.len() == 0 {
            return;
        }

        let point_timestamp = point.timestamp;

        // Propagate the h_res value to the first low_res archive
        self.downsample(h_res, rest[0], point_timestamp).map(|write_op| {
            self.perform_write_op(&write_op)
        });

        let high_res_iter = rest[0..rest.len()-1].into_iter();
        let low_res_iter = rest[1..].into_iter();

        let trickle_iter = high_res_iter.
            zip(low_res_iter).
            take_while(|&(h_res,l_res)| {
                match self.downsample(h_res, l_res, point_timestamp) {
                    Some(write_op) => {
                        self.perform_write_op(&write_op);
                        true
                    },
                    None => false
                }
            }).
            map(|_| ());

        // Don't need to collect anything, just force evaluation
        for _ in trickle_iter  {
            ();
        }
    }

    // TODO convert to return value to Result<WriteOp> so we can log why an update couldn't be done
    fn downsample(&self, h_res_archive: &ArchiveInfo,
                         l_res_archive: &ArchiveInfo,
                         point_timestamp: u64)
        -> Option<WriteOp> {

        // allocate space for all necessary points from higher archive
        // TODO: reuse buffer by passing it in from caller and here we can clear/push the buffer.
        // not positive it'll be faster
        let mut h_res_points = {
            let h_res_points_needed = l_res_archive.seconds_per_point / h_res_archive.seconds_per_point;
            vec![point::Point{timestamp: 0, value: 0.0}; h_res_points_needed as usize]
        };

        {
            // plan reads
            let reads = self.read_ops(
                h_res_archive, l_res_archive,
                &mut h_res_points[..],
                point_timestamp
            );

            // perform reads
            let ((first_index, first_buf), second_read) = reads;
            {
                let file = self.handle.borrow_mut();
                h_res_archive.read_points(first_index, first_buf, file);
            }

            match second_read {
                Some((second_index, second_buf)) => {
                    let file = self.handle.borrow_mut();
                    h_res_archive.read_points(second_index, second_buf, file);
                },
                None => ()
            }
        }

        // filter out of date samples
        let total_possible_values = h_res_points.len();
        let filtered_values = {
            let expected_timestamps = {
                let timestamp_start = l_res_archive.bucket(point_timestamp).0;
                let timestamp_stop = timestamp_start + (h_res_points.len() as u64)*h_res_archive.seconds_per_point;
                let step = h_res_archive.seconds_per_point;

                range_step_inclusive(timestamp_start, timestamp_stop, step)
            };
            self.filter_values(h_res_points, expected_timestamps)
        };

        // perform aggregation
        let aggregated_value = self.aggregate_samples_consume(filtered_values, total_possible_values as u64);
        aggregated_value.map(|aggregate| {
            let l_interval_start = l_res_archive.bucket(point_timestamp).0;
            let l_res_anchor = l_res_archive.anchor_bucket(self.handle.borrow_mut());
            let l_res_point = point::Point{ timestamp: l_interval_start, value: aggregate };
            build_write_op(l_res_archive, &l_res_point, l_res_anchor)
        })

        // write data
    }

    fn filter_values(&self, points: Vec<point::Point>, range: RangeStepInclusive<u64>) -> Vec<point::Point> {
        let filtered_values : Vec<point::Point> = points.into_iter().zip(range).
            filter_map(|(point, expected_timestamp)| {
                if point.timestamp == expected_timestamp {
                    Some(point)
                } else {
                    None
                }
            }).collect();
        filtered_values
    }

    // TODO: reading the points at the archive_info offset is done more than once (confirm that)
    // which means it can be cached and should be loaded to ArchiveInfo when those are parsed. How cool is that?
    fn read_ops<'a> (&'a self, h_res_archive: &ArchiveInfo,
                                 l_res_archive: &ArchiveInfo,
                                 h_res_points: &'a mut [point::Point],
                                 point_timestamp: u64)
        -> ((ArchiveIndex, &mut [point::Point]), Option<(ArchiveIndex, &mut [point::Point])>) {

        let h_anchor_bucket = BucketName( self.read_point(h_res_archive.offset).timestamp );
        puur::read_ops(h_res_archive, l_res_archive, h_res_points, point_timestamp, h_anchor_bucket)

    }

    fn aggregate_samples_consume(&self, valid_points: Vec<point::Point>, points_possible: u64) -> Option<f64>{
        let ratio : f32 = valid_points.len() as f32 / points_possible as f32;
        if ratio < self.header.metadata.x_files_factor {
            return None;
        }

        // TODO: we only do aggregation right now!
        match self.header.metadata.aggregation_type {
            AggregationType::Average => {
                let valid_points_len = valid_points.len();
                let sum = valid_points.into_iter().map(|p| p.value).fold(0.0, |l, r| l + r);
                Some(sum / valid_points_len as f64)
            },
            _ => { Some(0.0) }
        }
    }

    // TODO: don't create new vectors, borrow slice from archive_infos
    fn find_highest_res_archive(&self, current_time: u64, point_timestamp: u64) -> Option<(&ArchiveInfo, Vec<&ArchiveInfo>)>  {
        let mut archive_iter = self.header.archive_infos.iter();
        
        let high_precision_archive_option = archive_iter.find(|ai| {
            ai.retention > (current_time - point_timestamp)
        });

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

fn build_write_op(archive_info: &ArchiveInfo, point: &point::Point, archive_anchor: BucketName) -> WriteOp {
    let mut output_data = [0; 12];
    let bucket = archive_info.bucket(point.timestamp);
    {
        let point_value = point.value;
        let buf : &mut [u8] = &mut output_data;
        point::fill_buf(buf, bucket, point_value);
    }

    let seek_info = archive_info.calculate_seek(&point, archive_anchor);

    return WriteOp {
        seek: seek_info,
        bytes: output_data
    }
}

pub mod puur {
    use whisper::file::archive_info::{ ArchiveInfo, ArchiveIndex, BucketName };
    use whisper::point;

    pub fn read_ops<'a> (h_res_archive: &ArchiveInfo, 
                           l_res_archive: &ArchiveInfo,
                           h_res_points: &'a mut [point::Point],
                           point_timestamp: u64, h_res_anchor: BucketName)
    -> ((ArchiveIndex, &'a mut [point::Point]), Option<(ArchiveIndex, &'a mut [point::Point])>) {
        let h_res_start_index : ArchiveIndex = {
            if h_res_anchor.0 == 0 {
                ArchiveIndex(0)
            } else {
                let l_interval_start = l_res_archive.bucket(point_timestamp);
                let timespan  = l_interval_start.0 as i64 - h_res_anchor.0 as i64;
                let points = timespan / h_res_archive.seconds_per_point as i64;

                // TODO: Work around for modulo not working the same as in python.
                // TODO: OMG, move this craziness somewhere else
                let wrapped_index = {
                    let remainder = points % h_res_archive.points as i64;

                    if remainder < 0 {
                        h_res_archive.points as i64 + remainder
                    } else {
                        remainder
                    }
                };
                ArchiveIndex(wrapped_index as u64)
            }
        };

        let h_res_end_index = {
            let h_res_points_needed = l_res_archive.seconds_per_point / h_res_archive.seconds_per_point;
            let index = (h_res_start_index.0 + h_res_points_needed) % h_res_archive.points;
            ArchiveIndex(index)
        };

        // Contiguous read. The easy one.
        if h_res_start_index < h_res_end_index {
            ((h_res_start_index, &mut h_res_points[..]), None)
        // Wrap-around read
        } else {
            let (first_buf, second_buf) = h_res_points.split_at_mut((h_res_archive.points - h_res_start_index.0) as usize);
            let zero_index = ArchiveIndex(0);
            ((h_res_start_index, first_buf), Some((zero_index, second_buf)))
        }
    }
}

#[cfg(test)]
mod tests {
    use test::Bencher;
    extern crate time;

    use std::io::SeekFrom;

    use super::super::archive_info::{ ArchiveInfo, ArchiveIndex, BucketName };
    use super::{ WhisperFile, build_write_op, open };
    use super::puur;
    use whisper::point::Point;
    use whisper::schema::{ Schema, RetentionPolicy };
    use whisper::file::metadata::{ Metadata, AggregationType };

    fn build_60_1440_wsp(prefix: &str) -> WhisperFile {
        let path = format!("test/fixtures/{}.wsp", prefix);
        let schema = Schema {
            retention_policies: vec![
                RetentionPolicy {
                    precision: 60,
                    retention: 1440
                }
            ]
        };

        WhisperFile::new(&path[..], schema).unwrap()
    }

    fn build_60_1440_1440_168_10080_52(prefix: &str) -> WhisperFile {
        let path = format!("test/fixtures/{}.wsp", prefix);
        let specs = vec![
            "1m:1h".to_string(),
            "1h:1w".to_string(),
            "1w:1y".to_string()
        ];
        let schema = Schema::new_from_retention_specs(specs);

        WhisperFile::new(&path[..], schema).unwrap()
    }

    #[bench]
    fn bench_build_write_op(b: &mut Bencher) {
        let archive_info = ArchiveInfo {
            offset: SeekFrom::Start(28),
            seconds_per_point: 60,
            points: 1000,
            retention: 10000
        };
        let point = Point {
            timestamp: 1000,
            value: 10.0
        };
        b.iter(|| {
            let anchor_bucket = BucketName(900);
            build_write_op(&archive_info, &point, anchor_bucket)
        });
    }

    #[bench]
    fn bench_opening_a_file(b: &mut Bencher) {
        let path = "test/fixtures/60-1440.wsp";
        // TODO: how is this so fast? 7ns seems crazy. caching involved?
        b.iter(|| open(path).unwrap() );
    }

    #[bench]
    fn bench_writing_through_a_small_file(b: &mut Bencher) {
        let mut whisper_file = build_60_1440_wsp("small_file");
        let current_time = time::get_time().sec as u64;

        b.iter(|| {
            let point = Point {
                timestamp: current_time,
                value: 10.0
            };
            whisper_file.write(current_time, point);
        });
    }

    #[bench]
    fn bench_writing_through_a_large_file(b: &mut Bencher) {
        let mut whisper_file = build_60_1440_1440_168_10080_52("a_large_file");
        let current_time = time::get_time().sec as u64;

        b.iter(|| {
            let point = Point {
                timestamp: current_time,
                value: 10.0
            };
            whisper_file.write(current_time, point);
        });
    }

    // Take for example the active bucket layout for two archives
    // starting at time 100
    //
    // 1s archive with one wrap-around
    // +-----------------------------------------------------------+
    // |100|101|102|103|104|105|106|107|108|109|110|111|112|113|114|
    // +-----------------------------------------------------------+
    // |115|116|117|118|119|120|121|122|123|124|125|126|127|128|129|
    // +-----------------------------------------------------------+
    //
    // The second, 10s archive looks like this:
    // +-----------------------------------------------------------+
    // |010|020|030|040|050|060|070|080|090|100|110|120|130|140|150|
    // +-----------------------------------------------------------+
    //
    // Now let's shift our focus to handling a write for time 119.
    // 
    // the 1s archive at time 119, due to wrap-around, looks something like this:
    // +-----------------------------------------------------------+
    // |115|116|117|118|119|105|106|107|108|109|110|111|112|113|114|
    // +-----------------------------------------------------------+
    //
    // the 10s archive looks as before because it hasn't wrapped around yet:
    // +-----------------------------------------------------------+
    // |010|020|030|040|050|060|070|080|090|100|110|...|...|...|...|
    // +-----------------------------------------------------------+
    //
    // At time 119 you are downsampling for the second archive's
    // 110 slot. You need to consider all slots from the 1s archive
    // which map to the 110s slot, namely: 110s-119s.
    //
    // Let's annotate the 1s archive with array indexes:
    // +-----------------------------------------------------------+
    // | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10| 11| 12| 13| 14|
    // +-----------------------------------------------------------+
    // |115|116|117|118|119|105|106|107|108|109|110|111|112|113|114|
    // +-----------------------------------------------------------+
    //
    // To downsample to the 10s archive for slot 110 we need the
    // 1s slots for 110 to 119. Which becomes indexes 10 to 14 and 0 to 4.
    // The order matters, we want to to fill one pre-allocated buffer with the contiguous
    // values so we can pass them back to the caller and hide this wrap-around
    // complexity.
    //
    #[test]
    fn test_split_read_ops() {
        let h_res_archive = ArchiveInfo {
            offset: SeekFrom::Start(0),
            seconds_per_point: 1,
            points: 15,
            retention: 15 // 15 seconds
        };
        let l_res_archive = ArchiveInfo {
            offset: SeekFrom::Start(60*12),
            seconds_per_point: 10,
            retention: 75, // 1 minutes 15 seconds
            points: 15
        };

        let h_res_points_needed = l_res_archive.seconds_per_point / h_res_archive.seconds_per_point;
        assert_eq!(h_res_points_needed, 10);

        let mut h_res_points : Vec<Point> = vec![Point{timestamp: 0, value: 0.0}; h_res_points_needed as usize];

        let ((first_index,first_buf), second_op) = puur::read_ops(
            &h_res_archive, &l_res_archive,
            &mut h_res_points[..],
            119,  /* new point's timestamp */
            BucketName(100) /* high res archive's anchor bucket */
        );

        assert_eq!(first_index, ArchiveIndex(10));
        assert_eq!(first_buf.len(), 5);

        assert!(second_op.is_some());
        match second_op {
            Some((second_index,second_buf)) => {
                assert_eq!(second_buf.len(), 5);
                assert_eq!(second_index, ArchiveIndex(0) );
            },
            None => panic!("shouldn't happen!")
        };
    }


    #[test]
    fn test_contiguous_read_ops() {
        let h_res_archive = ArchiveInfo {
            offset: SeekFrom::Start(0),
            seconds_per_point: 1,
            points: 15,
            retention: 15 // 15 seconds
        };
        let l_res_archive = ArchiveInfo {
            offset: SeekFrom::Start(60*12),
            seconds_per_point: 10,
            retention: 75, // 1 minutes 15 seconds
            points: 15
        };

        let h_res_points_needed = l_res_archive.seconds_per_point / h_res_archive.seconds_per_point;
        assert_eq!(h_res_points_needed, 10);

        let mut h_res_points : Vec<Point> = vec![Point{timestamp: 0, value: 0.0}; h_res_points_needed as usize];

        let ((first_index,first_buf),second_read) = puur::read_ops(
            &h_res_archive, &l_res_archive,
            &mut h_res_points[..],
            102, /* new point's timestamp */
            BucketName(100) /* file's base timestamp */
        );

        assert_eq!(first_index, ArchiveIndex(0));
        assert_eq!(first_buf.len(), 10);
        assert_eq!(second_read, None);
    }

    #[test]
    fn test_read_point() {
        let file = open("test/fixtures/60-1440.wsp").unwrap();
        let offset = file.header.archive_infos[0].offset;
        // read the first point of the first archive
        let point = file.read_point(offset);
        assert_eq!(point, Point{timestamp: 0, value: 0.0});
    }

    #[test]
    fn test_read_points() {
        let file = open("test/fixtures/60-1440.wsp").unwrap();
        let offset = file.header.archive_infos[0].offset;
        // read the first point of the first archive

        let mut points_holder : Vec<Point> = vec![ Point{ timestamp: 0, value: 0.0 }; 10 ];
        file.read_points(offset, &mut points_holder[..]);

        let expected = vec![Point{timestamp: 0, value: 0.0}; points_holder.len()];
        assert_eq!(points_holder, expected);
    }

    #[test]
    fn test_new_file_has_correct_metadata() {
        let specs = vec![
            "1m:1h".to_string(),
            "1h:1w".to_string(),
            "1w:1y".to_string()
        ];
        let schema = Schema::new_from_retention_specs(specs);

        let file = WhisperFile::new("test/fixtures/new_has_correct_metadata.wsp", schema).unwrap();
        let header = file.header;

        let expected_metadata = Metadata {
            aggregation_type: AggregationType::Average,
            max_retention: 60*60*24*365,
            x_files_factor: 0.5,
            archive_count: 3
        };
        assert_eq!(header.metadata, expected_metadata);

        let archive_infos = header.archive_infos;
        let expected_archive_infos = vec![
            // 1m:1h
            ArchiveInfo {
                offset: SeekFrom::Start(52),
                seconds_per_point: 60,
                retention: 60*60,
                points: 60,
            },
            // 1h:1w
            ArchiveInfo {
                offset: SeekFrom::Start(52 + 60*12),
                seconds_per_point: 60*60,
                retention: 60*60*24*7,
                points: 24*7
            },
            // 1w:1y
            ArchiveInfo {
                offset: SeekFrom::Start(52 + 60*12 + 24*7*12),
                seconds_per_point: 60*60*24*7,
                retention: 60*60*24*365,
                points: 52
            }
        ];
        assert_eq!(archive_infos.len(), expected_archive_infos.len());
        assert_eq!(archive_infos[0], expected_archive_infos[0]);
        assert_eq!(archive_infos[1], expected_archive_infos[1]);
        assert_eq!(archive_infos[2], expected_archive_infos[2]);
    }


    #[test]
    fn test_split_first_archive() {
        let file = open("test/fixtures/60-1440-1440-168-10080-52.wsp").unwrap();
        let current_time = time::get_time().sec as u64;
        let point_timestamp = current_time - 100;
        let (best,rest) = file.find_highest_res_archive(current_time, point_timestamp).unwrap();

        let expected_best = ArchiveInfo {
            offset: SeekFrom::Start(52),
            seconds_per_point: 60,
            points: 1440,
            retention: 86400,
        };

        let expected_rest = vec![
            ArchiveInfo {
                offset: SeekFrom::Start(17332),
                seconds_per_point: 1440,
                points: 168,
                retention: 241920
            },
            ArchiveInfo {
                offset: SeekFrom::Start(19348),
                seconds_per_point: 10080,
                points: 52,
                retention: 524160
            }
        ];

        // Silly Vec<&T> makes this annoying. See TODO to change to slices.
        assert_eq!(rest.len(), 2);
        assert_eq!(*(rest[0]), expected_rest[0]);
        assert_eq!(*(rest[1]), expected_rest[1]);

        assert_eq!(*best, expected_best);
    }

    #[test]
    fn test_split_second_archive() {
        let file = open("test/fixtures/60-1440-1440-168-10080-52.wsp").unwrap();
        let current_time = time::get_time().sec as u64;

        // one sample past the first archive's retention
        let point_timestamp = current_time - 60*1441;

        let (best,rest) = file.find_highest_res_archive(current_time, point_timestamp).unwrap();

        let expected_best = ArchiveInfo {
            offset: SeekFrom::Start(17332),
            seconds_per_point: 1440,
            points: 168,
            retention: 241920
        };

        let expected_rest = vec![
            ArchiveInfo {
                offset: SeekFrom::Start(19348),
                seconds_per_point: 10080,
                points: 52,
                retention: 524160
            }
        ];

        // Silly Vec<&T> makes this annoying. See TODO to change to slices.
        assert_eq!(rest.len(), 1);
        assert_eq!(*(rest[0]), expected_rest[0]);

        assert_eq!(*best, expected_best);
    }

    #[test]
    fn test_split_no_archive() {
        let file = open("test/fixtures/60-1440-1440-168-10080-52.wsp").unwrap();
        let current_time = time::get_time().sec as u64;

        // one sample past the first archive's retention
        let point_timestamp = current_time - 10080*53;

        let split = file.find_highest_res_archive(current_time, point_timestamp);
        assert!(split.is_none());
    }
}
