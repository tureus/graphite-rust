This crate provides the graphite ecosystem of tools.

[![Build status](https://api.travis-ci.org/tureus/graphite-rust.png)](https://travis-ci.org/tureus/graphite-rust)

## Building

  $ git clone git@github.com:tureus/graphite-rust.git
  $ cd graphite-rust
  $ cargo build
  $ RUST_LOG=debug ./target/debug/carbon

## Tasks

 - [X] Read headers
 - [X] Read single point
 - [X] Write to single archive
 - [X] Write through all archives with downsampling
 - [X] Create files
 - [ ] Read many points
 - [ ] Lock files
 - [ ] Cache data for sampling (PROFILING)
 - [ ] `mmap` files (PROFILING)
 - [ ] UDP daemon
 - [ ] TCP daemon
 - [ ] Pickle daemon
 - [ ] HTTP frontend
 - [ ] Make logging useful for ops

## Documentation

[http://tureus.github.io/graphite-rust](http://tureus.github.io/graphite-rust)

## Reference

Documentation for the whisper file format is slim/nil. Clone the official repo and take a look at `whisper.py`

  $ git clone git@github.com:graphite-project/whisper.git
