#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::helpers::codecs::ChunkedDecoder;
use codec::helpers::codecs::PaddedDecoder;
use codec::helpers::codecs::PaddedEncoder;
use codec::helpers::codecs::StatelessEncoder;
use codec::CodecSettings;
use codec::CodecTransform;
use codec::Direction;
use codec::Error;
use codec::TransformableCodec;
use std::cmp;
use std::collections::BTreeMap;
use std::io;

#[derive(Default)]
pub struct Base32TransformFactory {}

#[derive(Default)]
pub struct Base32HexTransformFactory {}

pub const BASE32: [u8; 32] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'2', b'3', b'4', b'5', b'6', b'7',
];

pub const BASE32HEX: [u8; 32] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F',
    b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T', b'U', b'V',
];
pub const REV: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, 0, -1, -1, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8,
    9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

pub const REVHEX: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -1, -1, 0, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16, 17, 18,
    19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

fn forward_transform(inp: &[u8], outp: &mut [u8], arr: &[u8; 32]) -> (usize, usize) {
    let (is, os) = (5, 8);
    let bits = is * 8 / os;
    let mask = (1u64 << bits) - 1;
    let n = cmp::min(inp.len() / is, outp.len() / os);
    for (i, j) in (0..n).map(|x| (x * is, x * os)) {
        let x: u64 = inp[i..i + is]
            .iter()
            .enumerate()
            .map(|(k, &v)| u64::from(v) << ((is - 1 - k) * 8))
            .sum();

        for (k, val) in outp[j..j + os].iter_mut().enumerate().take(os) {
            *val = arr[(x >> ((os - 1 - k) * bits) & mask) as usize];
        }
    }
    (n * is, n * os)
}

impl Base32TransformFactory {
    pub fn new() -> Self {
        Base32TransformFactory {}
    }

    fn factory_for(
        name: &'static str,
        forward: &'static [u8; 32],
        reverse: &'static [i8; 256],
        r: Box<io::BufRead>,
        s: CodecSettings,
    ) -> Box<io::BufRead> {
        match s.dir {
            Direction::Forward => PaddedEncoder::new(
                StatelessEncoder::new(move |inp, out| forward_transform(inp, out, forward), 8),
                5,
                8,
                Some(b'='),
            )
            .into_bufread(r, s.bufsize),
            Direction::Reverse => PaddedDecoder::new(
                ChunkedDecoder::new(s.strict, name, 8, 5, reverse),
                8,
                5,
                Some(b'='),
            )
            .into_bufread(r, s.bufsize),
        }
    }
}

impl CodecTransform for Base32TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        Ok(Base32TransformFactory::factory_for(
            self.name(),
            &BASE32,
            &REV,
            r,
            s,
        ))
    }

    fn options(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "base32"
    }
}

impl Base32HexTransformFactory {
    pub fn new() -> Self {
        Base32HexTransformFactory {}
    }
}

impl CodecTransform for Base32HexTransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        Ok(Base32TransformFactory::factory_for(
            self.name(),
            &BASE32HEX,
            &REVHEX,
            r,
            s,
        ))
    }

    fn options(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "base32hex"
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(name: &str, inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
        }

        let rev = format!("-{}", name);
        for i in vec![8, 9, 10, 11, 512] {
            let c = Chain::new(&reg, &rev, i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, &rev, i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes_base32() {
        check("base32", b"", b"");
        check("base32", b"f", b"MY======");
        check("base32", b"fo", b"MZXQ====");
        check("base32", b"foo", b"MZXW6===");
        check("base32", b"foob", b"MZXW6YQ=");
        check("base32", b"fooba", b"MZXW6YTB");
        check("base32", b"foobar", b"MZXW6YTBOI======");
    }

    #[test]
    fn encodes_bytes_base32hex() {
        check("base32hex", b"", b"");
        check("base32hex", b"f", b"CO======");
        check("base32hex", b"fo", b"CPNG====");
        check("base32hex", b"foo", b"CPNMU===");
        check("base32hex", b"foob", b"CPNMUOG=");
        check("base32hex", b"fooba", b"CPNMUOJ1");
        check("base32hex", b"foobar", b"CPNMUOJ1E8======");
    }

    #[test]
    fn default_tests_base32() {
        tests::round_trip("base32");
        tests::basic_configuration("base32");
        tests::invalid_data("base32");
    }

    #[test]
    fn default_tests_base32hex() {
        tests::round_trip("base32hex");
        tests::basic_configuration("base32hex");
        tests::invalid_data("base32hex");
    }

    #[test]
    fn known_values() {
        check("base32", tests::BYTE_SEQ, b"AAAQEAYEAUDAOCAJBIFQYDIOB4IBCEQTCQKRMFYYDENBWHA5DYPSAIJCEMSCKJRHFAUSUKZMFUXC6MBRGIZTINJWG44DSOR3HQ6T4P2AIFBEGRCFIZDUQSKKJNGE2TSPKBIVEU2UKVLFOWCZLJNVYXK6L5QGCYTDMRSWMZ3INFVGW3DNNZXXA4LSON2HK5TXPB4XU634PV7H7AEBQKBYJBMGQ6EITCULRSGY5D4QSGJJHFEVS2LZRGM2TOOJ3HU7UCQ2FI5EUWTKPKFJVKV2ZLNOV6YLDMVTWS23NN5YXG5LXPF5X274BQOCYPCMLRWHZDE4VS6MZXHM7UGR2LJ5JVOW27MNTWW33TO55X7A4HROHZHF43T6R2PK5PWO33XP6DY7F47U6X3PP6HZ7L57Z7P674======");
        check("base32hex", tests::BYTE_SEQ, b"000G40O40K30E209185GO38E1S8124GJ2GAHC5OO34D1M70T3OFI08924CI2A9H750KIKAPC5KN2UC1H68PJ8D9M6SS3IEHR7GUJSFQ085146H258P3KGIAA9D64QJIFA18L4KQKALB5EM2PB9DLONAUBTG62OJ3CHIMCPR8D5L6MR3DDPNN0SBIEDQ7ATJNF1SNKURSFLV7V041GA1O91C6GU48J2KBHI6OT3SGI699754LIQBPH6CQJEE9R7KVK2GQ58T4KMJAFA59LALQPBDELUOB3CLJMIQRDDTON6TBNF5TNQVS1GE2OF2CBHM7P34SLIUCPN7CVK6HQB9T9LEMQVCDJMMRRJETTNV0S7HE7P75SRJUHQFATFMERRNFU3OV5SVKUNRFFU7PVBTVPVFUVS======");
    }
}
