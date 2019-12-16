/// A variety of useful test case generators.
use chain::Chain;
use codec::registry::CodecRegistry;
use codec::{CodecSettings, CodecTransform, Direction, Error};
use rand_chacha::ChaChaRng;
use rand_core::{RngCore, SeedableRng};
use std::collections::BTreeMap;
use std::env;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

// Fixed constants for use in tests.
pub const BYTE_SEQ : &[u8] = b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2a\x2b\x2c\x2d\x2e\x2f\x30\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\x3e\x3f\x40\x41\x42\x43\x44\x45\x46\x47\x48\x49\x4a\x4b\x4c\x4d\x4e\x4f\x50\x51\x52\x53\x54\x55\x56\x57\x58\x59\x5a\x5b\x5c\x5d\x5e\x5f\x60\x61\x62\x63\x64\x65\x66\x67\x68\x69\x6a\x6b\x6c\x6d\x6e\x6f\x70\x71\x72\x73\x74\x75\x76\x77\x78\x79\x7a\x7b\x7c\x7d\x7e\x7f\x80\x81\x82\x83\x84\x85\x86\x87\x88\x89\x8a\x8b\x8c\x8d\x8e\x8f\x90\x91\x92\x93\x94\x95\x96\x97\x98\x99\x9a\x9b\x9c\x9d\x9e\x9f\xa0\xa1\xa2\xa3\xa4\xa5\xa6\xa7\xa8\xa9\xaa\xab\xac\xad\xae\xaf\xb0\xb1\xb2\xb3\xb4\xb5\xb6\xb7\xb8\xb9\xba\xbb\xbc\xbd\xbe\xbf\xc0\xc1\xc2\xc3\xc4\xc5\xc6\xc7\xc8\xc9\xca\xcb\xcc\xcd\xce\xcf\xd0\xd1\xd2\xd3\xd4\xd5\xd6\xd7\xd8\xd9\xda\xdb\xdc\xdd\xde\xdf\xe0\xe1\xe2\xe3\xe4\xe5\xe6\xe7\xe8\xe9\xea\xeb\xec\xed\xee\xef\xf0\xf1\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9\xfa\xfb\xfc\xfd\xfe\xff";

// Test helpers.
fn prng(time: bool) -> ChaChaRng {
    let seed = if time {
        if let Ok(d) = SystemTime::now().duration_since(UNIX_EPOCH) {
            d.as_secs() / 86400
        } else {
            0
        }
    } else {
        u64::from_str_radix(&env::var("RAND_SEED").unwrap_or("0".to_string()), 10).unwrap_or(0)
    };
    ChaChaRng::seed_from_u64(seed)
}

pub fn round_trip(name: &'static str) {
    for p in &[true, false] {
        let mut prng = prng(*p);
        for i in 0..512 {
            round_trip_with_prng(name, &mut prng, i);
        }
    }
    for i in 0..512 {
        round_trip_with_fill(name, i);
    }
    round_trip_with_fill(name, 32768);
    round_trip_bytes(name, BYTE_SEQ, "all-bytes");
}

fn round_trip_with_prng(name: &'static str, rng: &mut RngCore, sz: usize) {
    let mut v = vec![0u8; sz];
    rng.fill_bytes(v.as_mut_slice());
    round_trip_bytes(name, &v, "random");
}

fn round_trip_with_fill(name: &'static str, sz: usize) {
    let v = vec![sz as u8; sz];
    round_trip_bytes(name, &v, "fill");
}

fn round_trip_bytes(name: &'static str, inp: &[u8], desc: &str) {
    let reg = CodecRegistry::new();
    let reverse = format!("-{}", name);
    for &i in &[64, 65, 66, 67, 512] {
        let c = Chain::new(&reg, name, i, true);
        let outp = c.transform(inp.to_vec()).unwrap();
        let c = Chain::new(&reg, &reverse, i, true);
        assert_eq!(
            c.transform(outp.to_vec()).unwrap(),
            inp,
            "round-trip {} ({} bytes, {}-byte chunks, {})",
            name,
            inp.len(),
            i,
            desc
        );
        let c = Chain::new(&reg, &reverse, i, false);
        assert_eq!(
            c.transform(outp.to_vec()).unwrap(),
            inp,
            "round-trip {} ({} bytes, {}-byte chunks, {}, strict)",
            name,
            inp.len(),
            i,
            desc
        );
    }
}

pub fn invalid_data(name: &str) {
    for p in &[true, false] {
        let mut prng = prng(*p);
        for i in 0..512 {
            invalid_data_with_prng(name, &mut prng, i);
        }
    }
    for i in 0..512 {
        invalid_data_with_fill(name, i);
    }
}

fn invalid_data_with_prng(name: &str, rng: &mut RngCore, sz: usize) {
    let mut v = vec![0u8; sz];
    rng.fill_bytes(v.as_mut_slice());
    invalid_data_bytes(name, &v);
}

fn invalid_data_with_fill(name: &str, sz: usize) {
    let v = vec![sz as u8; sz];
    invalid_data_bytes(name, &v);
}

#[allow(unused_must_use)]
fn invalid_data_bytes(name: &str, inp: &[u8]) {
    let reg = CodecRegistry::new();
    let reverse = format!("-{}", name);

    // This is a test for not panicking.  We don't mandate an error because it's possible that some
    // byte sequences may happen to coincide with valid encodings, especially for short sequences.
    let c = Chain::new(&reg, &reverse, 512, true);
    c.transform(inp.to_vec());
    let c = Chain::new(&reg, &reverse, 512, false);
    c.transform(inp.to_vec());
}

pub fn basic_configuration(name: &str) {
    let reg = CodecRegistry::new();

    let transform = match reg.iter().find(|&(&k, _)| k == name) {
        Some((_, v)) => v.as_ref(),
        None => panic!("Can't find {}", name),
    };

    assert_eq!(
        transform.name(),
        name,
        "transform has expected name: {}",
        name
    );

    let settings = CodecSettings {
        bufsize: 8192,
        strict: true,
        args: BTreeMap::new(),
        dir: Direction::Reverse,
    };
    if transform.can_reverse() {
        match instantiate(transform, settings) {
            Ok(_) => (),
            Err(Error::MissingArgument(_)) => (),
            Err(e) => panic!("Can't instantiate reverse transform: {}", e),
        };
    } else {
        match instantiate(transform, settings) {
            Ok(_) => panic!("Successfully instantiated unreversible codec"),
            Err(Error::ForwardOnly(_)) => (),
            Err(e) => panic!("Unexpected error instantiating reverse transform: {}", e),
        };
    }

    for (arg, _) in transform.options() {
        let mut args = BTreeMap::new();
        args.insert(arg, None);

        let settings = CodecSettings {
            bufsize: 8192,
            strict: true,
            args,
            dir: Direction::Forward,
        };

        instantiate(transform, settings).expect("Can instantiate with each arg");
    }
}

fn instantiate(
    transform: &CodecTransform,
    settings: CodecSettings,
) -> Result<Box<io::BufRead>, Error> {
    transform.factory(Box::new(io::Cursor::new("abc")), settings)
}
