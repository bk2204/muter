#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

extern crate blake2;
#[cfg(feature = "modern")]
extern crate blake3;
extern crate clap;
extern crate digest;
extern crate md5;
extern crate multi_reader;
extern crate muter;
#[cfg(test)]
extern crate rand_chacha;
#[cfg(test)]
extern crate rand_core;
extern crate sha1;
extern crate sha2;
extern crate sha3;
#[macro_use]
extern crate tr;

use muter::chain;
use muter::codec;
use muter::codec::registry::CodecRegistry;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::process;

use clap::{App, Arg, ArgMatches};

const BUFFER_SIZE: usize = codec::DEFAULT_BUFFER_SIZE;

fn source(values: Vec<&OsStr>, bufsize: usize) -> io::Result<Box<io::BufRead>> {
    if values.is_empty() {
        return Ok(Box::new(io::BufReader::with_capacity(bufsize, io::stdin())));
    }
    let files = values
        .iter()
        .map(fs::File::open)
        .collect::<Result<Vec<_>, io::Error>>()?;
    Ok(Box::new(io::BufReader::with_capacity(
        bufsize,
        multi_reader::MultiReader::new(files.into_iter()),
    )))
}

fn create_chain(reg: &CodecRegistry, m: ArgMatches) -> io::Result<Box<io::BufRead>> {
    let chain = m.value_of("chain").unwrap();
    let sources = match m.values_of_os("INPUT") {
        Some(x) => x.collect(),
        None => vec![],
    };
    let bufsize = match m
        .value_of("buffer-size")
        .map(|val| val.parse())
        .unwrap_or(Ok(BUFFER_SIZE))
    {
        Ok(x) => x,
        Err(_) => {
            return Err(muter::codec::Error::InvalidArgument(
                "buffer-size".to_string(),
                m.value_of("buffer-size").unwrap().to_string(),
            )
            .into())
        }
    };
    let mut c = chain::Chain::new(reg, chain, bufsize, !m.is_present("no-strict"));
    if m.is_present("reverse") {
        c = c.reverse();
    }
    c.build(source(sources, bufsize)?)
}

fn process(reg: &CodecRegistry, m: ArgMatches) -> io::Result<()> {
    let mut transform = create_chain(reg, m)?;
    std::io::copy(&mut transform, &mut io::stdout())?;
    Ok(())
}

fn help(reg: &CodecRegistry) -> String {
    let mut s: String = tr!("
Modify the bytes in the concatentation of INPUT (or standard input) by using the
specification in CHAIN.

CHAIN is a colon-separated list of encoding transform.  A transform can be
prefixed with - to reverse it (if possible).  A transform can be followed by one
or more comma-separated parenthesized arguments as well.  Instead of
parentheses, a single comma may be used.

For example, '-hex:hash(sha256):base64' (or '-hex:hash,sha256:base64') decodes a
hex-encoded string, hashes it with SHA-256, and converts the result to base64.

If --reverse is specified, reverse the order of transforms in order and in sense.

The following transforms are available:
");
    let mut v: Vec<String> = vec![];
    for (name, xfrm) in reg.iter() {
        v.push(format!("  {}", name));
        for (opt, desc) in xfrm.options() {
            v.push(format!("    {:10}: {}", opt, desc));
        }
    }
    s += &v.join("\n");
    s
}

fn main() {
    tr_init!(format!("{}/locale", env!("sharedir")));
    let reg = CodecRegistry::new();
    let help = help(&reg);
    let matches = App::new("muter")
        .about(tr!("Encodes and decodes byte sequences").as_str())
        .arg(
            Arg::with_name("chain")
                .short("c")
                .long("chain")
                .value_name(tr!("CHAIN").as_str())
                .help(tr!("List of transforms to perform").as_str())
                .required(true)
                .takes_value(true)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::with_name("reverse")
                .short("r")
                .long("reverse")
                .help(tr!("Reverse transforms in both order and direction").as_str()),
        )
        .arg(
            Arg::with_name("buffer-size")
                .long("buffer-size")
                .takes_value(true)
                .help(tr!("Size of buffer").as_str()),
        )
        .arg(
            Arg::with_name("strict")
                .long("strict")
                .help(tr!("Enable strict decoding (default)").as_str()),
        )
        .arg(
            Arg::with_name("no-strict")
                .long("no-strict")
                .conflicts_with("strict")
                .help(tr!("Enable non-strict decoding").as_str()),
        )
        .arg(
            Arg::with_name("INPUT")
                .help(tr!("Input files to process").as_str())
                .multiple(true)
                .index(1),
        )
        .after_help(&*help)
        .get_matches();
    if let Err(e) = process(&reg, matches) {
        if let Some(err) = e.get_ref() {
            eprintln!("muter: {}", err);
            process::exit(2);
        } else if e.kind() == io::ErrorKind::BrokenPipe {
            process::exit(141);
        } else {
            eprintln!("muter: {}", e);
            process::exit(3);
        }
    }
}
