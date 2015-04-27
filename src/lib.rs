/*!
This is a reimplementation of the graphite logging framework.
It aims to maintain total compatibility with features and file format
of your normal graphite installation. Just made easier by one
binary. And possibly higher performance.

Also, this is brand-new code. In the true rust spirit it does not guarantee
the safety of your kittens.
*/

#![crate_name = "graphite"]
extern crate time;
extern crate byteorder;

pub mod whisper;
