The graphite ecosystem in one easy-to-install package.

[![Build status](https://api.travis-ci.org/tureus/graphite-rust.png)](https://travis-ci.org/tureus/graphite-rust)

## Docker

By far the easiest experience for getting up and running

  docker pull xrlx/graphite
  mkdir data
  docker run -v ./data:/data xrlx/graphite

How I run `graphite-rust` with `graphite-web` in production:

    $ cat run_graphite.sh
    docker run -e "RUST_LOG=warning" --name graphite -d -p 2003:2003/udp -p 2003:2003 -v /var/data/graphite:/data xrlx/graphite
    $ cat run_graphite_web.sh
    docker run -d -it --name graphite-web -v /var/data/graphite:/opt/graphite/storage/whisper -p 80:80 banno/graphite-web

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
 - [ ] Advisory lock files
 - [x] `mmap` files (PROFILING)
  - [ ] Use `cfg()` guards to provide conditional checks for sysctl settings
 - [X] UDP daemon
 - [X] TCP daemon
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

  echo "local.random.diceroll 4 `date +%s`" | nc -u -w 1 localhost 2003

Memory stats:

  yum install -y sysstat
  toolbox sar -B 1
