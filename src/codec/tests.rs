/// A variety of useful test case generators.
use chain::Chain;
use codec::registry::CodecRegistry;
use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use std::time::SystemTime;

fn prng(time: bool) -> ChaCha20Rng {
    let seed = if time {
        if let Ok(d) = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            d.as_secs() / 86400
        } else {
            0
        }
    } else {
        u64::from_str_radix(&std::env::var("RAND_SEED").unwrap_or("0".to_string()), 10).unwrap_or(0)
    };
    ChaCha20Rng::seed_from_u64(seed)
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
}

fn round_trip_with_prng(name: &'static str, rng: &mut RngCore, sz: usize) {
    let mut v = vec![0u8; sz];
    rng.fill_bytes(v.as_mut_slice());
    round_trip_bytes(name, &v);
}

fn round_trip_with_fill(name: &'static str, sz: usize) {
    let v = vec![sz as u8; sz];
    round_trip_bytes(name, &v);
}

fn round_trip_bytes(name: &'static str, inp: &[u8]) {
    let reverse = format!("-{}", name);
    for i in vec![5, 6, 7, 8, 512] {
        let c = Chain::new(reg(), name, i, true);
        let outp = c.transform(inp.to_vec()).unwrap();
        let c = Chain::new(reg(), &reverse, i, true);
        assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        let c = Chain::new(reg(), &reverse, i, false);
        assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
    }
}

fn reg() -> CodecRegistry {
    CodecRegistry::new()
}
