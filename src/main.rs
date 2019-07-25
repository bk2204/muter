extern crate clap;
#[cfg(test)]
extern crate rand_chacha;
#[cfg(test)]
extern crate rand_core;
pub mod chain;
pub mod codec;

use codec::registry::CodecRegistry;
use std::io;
use std::process;

use clap::{App, Arg, ArgMatches};

const BUFFER_SIZE: usize = codec::DEFAULT_BUFFER_SIZE;

fn source() -> io::Result<Box<io::BufRead>> {
    Ok(Box::new(io::BufReader::with_capacity(
        BUFFER_SIZE,
        io::stdin(),
    )))
}

fn create_chain(m: ArgMatches) -> io::Result<Box<io::BufRead>> {
    let chain = m.value_of("chain").unwrap();
    let reg = CodecRegistry::new();
    let c = chain::Chain::new(&reg, chain, BUFFER_SIZE, true);
    c.build(source()?)
}

fn process(m: ArgMatches) -> io::Result<()> {
    let mut transform = create_chain(m)?;
    std::io::copy(&mut transform, &mut io::stdout())?;
    Ok(())
}

fn main() {
    let matches = App::new("muter")
        .about("Encodes and decodes byte sequence")
        .arg(
            Arg::with_name("chain")
                .short("c")
                .value_name("CHAIN")
                .help("List of transforms to perform")
                .required(true)
                .takes_value(true)
                .allow_hyphen_values(true),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Input files to process")
                .multiple(true)
                .index(1),
        )
        .get_matches();
    if let Err(e) = process(matches) {
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
