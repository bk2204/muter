use codec::Codec;
use codec::CodecSettings;
use codec::Direction;
use codec::Error;
use codec::FlushState;
use codec::StatelessEncoder;
use codec::Status;
use codec::Transform;
use std::io;

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
    pub fn factory(r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
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
}

pub struct Decoder {
    strict: bool,
}

impl Decoder {
    fn new(strict: bool) -> Self {
        Decoder { strict: strict }
    }
}

impl Codec for Decoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
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
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn reg() -> CodecRegistry {
        CodecRegistry::new()
    }

    fn check(inp: &[u8], lower: &[u8], upper: &[u8]) {
        let c = Chain::new(reg(), "hex", 512, true);
        assert_eq!(c.transform(inp.to_vec()).unwrap(), lower);
        let c = Chain::new(reg(), "hex,lower", 512, true);
        assert_eq!(c.transform(inp.to_vec()).unwrap(), lower);
        let c = Chain::new(reg(), "hex,upper", 512, true);
        assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
        let c = Chain::new(reg(), "-hex", 512, true);
        assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
        let c = Chain::new(reg(), "-hex", 512, true);
        assert_eq!(c.transform(lower.to_vec()).unwrap(), inp);
        let c = Chain::new(reg(), "-hex", 512, false);
        assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
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
}
