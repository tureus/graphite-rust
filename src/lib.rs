/*!

# graphite-rust

Herein lies a humble reimplementation of the graphite metrics system.
It's a time series database system, with frontend and management tools,
that strikes a reasonable balance between features, ease of management, and
a nice network of tools which can talk to graphite.

`graphite-rust` aims to maintain total compatibility with features and file format
of your normal graphite installation. Just made easier by one
binary. And possibly higher performance but that's not the main goal right now.
Although I am pursing a batch-write model out the gate -- but that's mostly to understand how to work with vectors.

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
extern crate time;
extern crate byteorder;

pub mod whisper;
