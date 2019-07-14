use codec::ChunkedDecoder;
use codec::CodecSettings;
use codec::Direction;
use codec::Error;
use codec::PaddedDecoder;
use codec::PaddedEncoder;
use codec::Transform;
use std::io;

pub struct TransformFactory {}

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
    let n = std::cmp::min(inp.len() / is, outp.len() / os);
    for (i, j) in (0..n).map(|x| (x * is, x * os)) {
        let x: u64 = inp[i..i + is]
            .iter()
            .enumerate()
            .map(|(k, &v)| (v as u64) << ((is - 1 - k) * 8))
            .sum();
        for k in 0..os {
            outp[j + k] = arr[(x >> ((os - 1 - k) * bits) & mask) as usize];
        }
    }
    (n * is, n * os)
}

impl TransformFactory {
    pub fn factory(r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => {
                let enc =
                    PaddedEncoder::new(move |inp, out| forward_transform(inp, out, &BASE32), 5, 8);
                Ok(Box::new(Transform::new(r, enc)))
            }
            Direction::Reverse => Ok(Box::new(Transform::new(
                r,
                PaddedDecoder::new(ChunkedDecoder::new(s.strict, "base32", 8, 5, &REV), 8, 5),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::codecs::base32::PaddedEncoder;
    use codec::registry::CodecRegistry;
    use codec::tests;

    #[test]
    fn pads_correctly() {
        let cases = vec![
            (5, 8, 0, 0),
            (5, 8, 1, 6),
            (5, 8, 2, 4),
            (5, 8, 3, 3),
            (5, 8, 4, 1),
            (3, 4, 0, 0),
            (3, 4, 1, 2),
            (3, 4, 2, 1),
        ];

        for (isize, osize, inbytes, padbytes) in cases {
            let p = PaddedEncoder::new(|_, _| (0, 0), isize, osize);
            assert_eq!(p.pad_bytes_needed(inbytes), padbytes);
        }
    }

    fn reg() -> CodecRegistry {
        CodecRegistry::new()
    }

    fn check(inp: &[u8], outp: &[u8]) {
        for i in vec![5, 6, 7, 8, 512] {
            let c = Chain::new(reg(), "base32", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(reg(), "-base32", i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(reg(), "-base32", i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes() {
        check(b"", b"");
        check(b"f", b"MY======");
        check(b"fo", b"MZXQ====");
        check(b"foo", b"MZXW6===");
        check(b"foob", b"MZXW6YQ=");
        check(b"fooba", b"MZXW6YTB");
        check(b"foobar", b"MZXW6YTBOI======");
    }

    #[test]
    fn default_tests() {
        tests::round_trip("base32");
    }
}