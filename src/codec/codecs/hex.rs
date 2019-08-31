#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::Codec;
use codec::CodecSettings;
use codec::CodecTransform;
use codec::Direction;
use codec::Error;
use codec::FlushState;
use codec::StatelessEncoder;
use codec::Status;
use codec::Transform;
use std::collections::BTreeMap;
use std::io;

#[derive(Default)]
pub struct TransformFactory {}

pub const LOWER: [u8; 16] = [
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
];
pub const UPPER: [u8; 16] = [
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46,
];

pub const REV: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 10,
    11, 12, 13, 14, 15, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

fn forward_transform(inp: &[u8], outp: &mut [u8], arr: &[u8; 16]) -> (usize, usize) {
    let n = std::cmp::min(inp.len(), outp.len() / 2);
    for (i, j) in (0..n).map(|x| (x, x * 2)) {
        outp[j + 0] = arr[(inp[i] >> 4) as usize];
        outp[j + 1] = arr[(inp[i] & 0xf) as usize];
    }
    (n, n * 2)
}

impl TransformFactory {
    pub fn new() -> Self {
        TransformFactory {}
    }
}

impl CodecTransform for TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => {
                let arr = match s.args.contains("upper") {
                    true => &UPPER,
                    false => &LOWER,
                };
                let enc = StatelessEncoder::new(move |inp, out| forward_transform(inp, out, arr));
                Ok(Box::new(Transform::new(r, enc)))
            }
            Direction::Reverse => Ok(Box::new(Transform::new(r, Decoder::new(s.strict)))),
        }
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        let mut map = BTreeMap::new();
        map.insert("lower".to_string(), "use lowercase letters");
        map.insert("upper".to_string(), "use uppercase letters");
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "hex"
    }
}

pub struct Decoder {
    strict: bool,
}

impl Decoder {
    fn new(strict: bool) -> Self {
        Decoder { strict }
    }
}

impl Codec for Decoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        match f {
            FlushState::None if src.len() < 2 => {
                return Ok(Status::BufError(0, 0));
            }
            FlushState::Finish if src.len() == 0 => {
                return Ok(Status::StreamEnd(0, 0));
            }
            _ => (),
        }

        let vec: Vec<(usize, u8)> = if self.strict {
            src.iter()
                .cloned()
                .enumerate()
                .take(dst.len() * 2)
                .collect()
        } else {
            src.iter()
                .cloned()
                .enumerate()
                .filter(|(_, x)| REV[*x as usize] != -1)
                .take(dst.len() * 2)
                .collect()
        };

        let bytes = vec.len() / 2;
        let mut n = 0;
        for (i, j) in (0..bytes).map(|x| (x * 2, x)) {
            let ((_, x), (b, y)) = (vec[i], vec[i + 1]);
            let v: i16 = ((REV[x as usize] as i16) << 4) | REV[y as usize] as i16;
            if v < 0 {
                return Err(Error::InvalidSequence("hex".to_string(), vec![x, y]));
            }
            dst[j] = (v & 0xff) as u8;
            n = b;
        }

        match f {
            FlushState::Finish if n == src.len() => Ok(Status::StreamEnd(n + 1, bytes)),
            _ => Ok(Status::Ok(n + 1, bytes)),
        }
    }

    fn chunk_size(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(inp: &[u8], lower: &[u8], upper: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, "hex", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), lower);
            let c = Chain::new(&reg, "hex,lower", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), lower);
            let c = Chain::new(&reg, "hex,upper", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
            let c = Chain::new(&reg, "-hex", i, true);
            assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-hex", i, true);
            assert_eq!(c.transform(lower.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-hex", i, false);
            assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes() {
        check(b"abc", b"616263", b"616263");
        check(b"\x00\xff", b"00ff", b"00FF");
        check(b"\xc2\xa9", b"c2a9", b"C2A9");
        check(
            b"\x01\x23\x45\x67\x89\xab\xcd\xef",
            b"0123456789abcdef",
            b"0123456789ABCDEF",
        );
        check(b"\xfe\xdc\xba", b"fedcba", b"FEDCBA");
    }

    #[test]
    fn default_tests() {
        tests::round_trip("hex");
        tests::round_trip("hex,upper");
        tests::round_trip("hex,lower");
    }

    #[test]
    fn known_values() {
        check(tests::BYTE_SEQ, b"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff", b"000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F202122232425262728292A2B2C2D2E2F303132333435363738393A3B3C3D3E3F404142434445464748494A4B4C4D4E4F505152535455565758595A5B5C5D5E5F606162636465666768696A6B6C6D6E6F707172737475767778797A7B7C7D7E7F808182838485868788898A8B8C8D8E8F909192939495969798999A9B9C9D9E9FA0A1A2A3A4A5A6A7A8A9AAABACADAEAFB0B1B2B3B4B5B6B7B8B9BABBBCBDBEBFC0C1C2C3C4C5C6C7C8C9CACBCCCDCECFD0D1D2D3D4D5D6D7D8D9DADBDCDDDEDFE0E1E2E3E4E5E6E7E8E9EAEBECEDEEEFF0F1F2F3F4F5F6F7F8F9FAFBFCFDFEFF");
    }
}
