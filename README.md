The graphite ecosystem in one easy-to-install package.

[![Build status](https://api.travis-ci.org/tureus/graphite-rust.png)](https://travis-ci.org/tureus/graphite-rust)

## Docker

By far the easiest experience for getting up and running

  docker pull xrlx/graphite
  mkdir data
  docker run -v ./data:/data xrlx/graphite

## Building

Note: you'll need a nightly rust build to build this

  $ git clone git@github.com:tureus/graphite-rust.git
  $ cd graphite-rust
  $ cargo build --release
  $ RUST_LOG=debug ./target/debug/carbon

## Tasks

 - [X] Read headers
 - [X] Read single point
 - [X] Write to single archive
 - [X] Write through all archives with downsampling
 - [X] Create files
 - [X] Read many points
 - [ND] Lock files (not necessary with memmap pattern?)
 - [ ] `mmap` files (PROFILING)
 - [X] UDP daemon
 - [ ] TCP daemon
 - [ ] Custom schema support when creating new WSPs
 - [ ] Pickle daemon
 - [ ] HTTP frontend
 - [ ] Make logging useful for ops
 - [ ] Validate .wsp when opening (archives need to cleanly multiply, etc)

## Documentation

[http://tureus.github.io/graphite-rust](http://tureus.github.io/graphite-rust)

## Reference

Documentation for the whisper file format is slim/nil. Clone the official repo and take a look at `whisper.py`

  $ git clone git@github.com:graphite-project/whisper.git

## Talking to Carbon

On OSX you need to specify IPv4:

  echo -e "local.random.diceroll 4 `date +%s`" | nc -4u -w0 localhost 2003

On linux:

  echo "local.random.diceroll 4 `date +%s`" | nc -u -w1 localhost 2003
