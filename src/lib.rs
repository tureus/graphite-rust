/*!

# graphite-rust

Herein lies a humble reimplementation of the graphite metrics system -- a time series database system,
with HTTP frontend and management tools.
It strikes a reasonable balance between features you need, easy management, and a rich ecosystem of tools which can talk to graphite.

`graphite-rust` aims to maintain total compatibility in both features and file formats.
The big selling point should be the ease of installation: it's just one
binary. It may be higher performance but that's not the main goal.

This work falls under the name `graphite` but in reality the package has distinct components you may want to understand:

 * [`whisper`](whisper/index.html) - all the heavy lifting for parsing and writing to whisper database files
 * `carbon` - the network daemon which mediates access to whisper files
 * `graphite` - the HTTP REST server which handles queries. It has a minimal HTML
    application for creating dashboard but I'll be skipping that. For dashboard you'll want [`grafana`](http://grafana.org/).

Status of the codes:

 * `whisper`
  * *DOES* open and parse header/metadata/archive info
  * *IN PROGRESS* take a metric value and provide a WriteOp
  * *NOT STARTED* write `vec![WriteOp]` to file
 * `carbon`
  * *DOESN'T DO ANYTHING*
 * `graphite`
  * *DOESN'T EXIST*

## Also, this is brand-new code. In the true rust spirit it does not guarantee the safety of your kittens.

*/

#![crate_name = "graphite"]
#![feature(trace_macros)]
#![feature(path_ext, dir_builder, slice_chars)]

#![feature(test)]
extern crate test;

extern crate time;
extern crate byteorder;

#[macro_use]
extern crate log;
extern crate env_logger;

extern crate num;

extern crate regex;

// Graphite server deps
extern crate iron;
extern crate router;
extern crate urlencoded;
extern crate glob;

pub mod whisper;
pub mod carbon;
pub mod graphite;
