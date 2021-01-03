#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

extern crate blake2;
extern crate clap;
extern crate digest;
extern crate flate2;
extern crate md5;
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;
extern crate rand_chacha;
extern crate rand_core;
extern crate sha1;
extern crate sha2;
extern crate sha3;
#[macro_use]
extern crate tr;
pub mod chain;
pub mod codec;
