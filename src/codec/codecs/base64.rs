use codec::ChunkedDecoder;
use codec::CodecSettings;
use codec::Direction;
use codec::Error;
use codec::PaddedDecoder;
use codec::PaddedEncoder;
use codec::Transform;
use std::io;

pub struct TransformFactory {}

pub const BASE64: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f',
    b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v',
    b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'+', b'/',
];

pub const REV: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1, -1, 63,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, 0, -1, -1, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8,
    9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, -1, -1, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

fn forward_transform(inp: &[u8], outp: &mut [u8], arr: &[u8; 64]) -> (usize, usize) {
    let (is, os) = (3, 4);
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
                    PaddedEncoder::new(move |inp, out| forward_transform(inp, out, &BASE64), 3, 4);
                Ok(Box::new(Transform::new(r, enc)))
            }
            Direction::Reverse => Ok(Box::new(Transform::new(
                r,
                PaddedDecoder::new(ChunkedDecoder::new(s.strict, "base64", 4, 3, &REV), 4, 3),
            ))),
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

    fn check(inp: &[u8], outp: &[u8]) {
        for i in vec![5, 6, 7, 8, 512] {
            let c = Chain::new(reg(), "base64", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(reg(), "-base64", i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(reg(), "-base64", i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes() {
        check(b"", b"");
        check(b"f", b"Zg==");
        check(b"fo", b"Zm8=");
        check(b"foo", b"Zm9v");
        check(b"foob", b"Zm9vYg==");
        check(b"fooba", b"Zm9vYmE=");
        check(b"foobar", b"Zm9vYmFy");
    }

    #[test]
    fn default_tests() {
        tests::round_trip("base64");
    }
}
