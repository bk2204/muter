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

pub const BASE64: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f',
    b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v',
    b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'+', b'/',
];

pub const URL64: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f',
    b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v',
    b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'-', b'_',
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

pub const URLREV: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
    52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, 0, -1, -1, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8,
    9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1, 63, -1, 26,
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

#[derive(Default)]
pub struct Base64TransformFactory {}

impl Base64TransformFactory {
    pub fn new() -> Self {
        Base64TransformFactory {}
    }
}

impl CodecTransform for Base64TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => Ok(PaddedEncoder::new(
                StatelessEncoder::new(move |inp, out| forward_transform(inp, out, &BASE64), 4),
                3,
                4,
                Some(b'='),
            )
            .into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(PaddedDecoder::new(
                ChunkedDecoder::new(s.strict, "base64", 4, 3, &REV),
                4,
                3,
                Some(b'='),
            )
            .into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "base64"
    }
}

#[derive(Default)]
pub struct URL64TransformFactory {}

impl URL64TransformFactory {
    pub fn new() -> Self {
        URL64TransformFactory {}
    }
}

impl CodecTransform for URL64TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => Ok(PaddedEncoder::new(
                StatelessEncoder::new(move |inp, out| forward_transform(inp, out, &URL64), 4),
                3,
                4,
                None,
            )
            .into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(PaddedDecoder::new(
                ChunkedDecoder::new(s.strict, "url64", 4, 3, &URLREV),
                4,
                3,
                None,
            )
            .into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "url64"
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(name: &str, inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        let rev = format!("-{}", name);
        for i in vec![5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, &rev, i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, &rev, i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes_base64() {
        check("base64", b"", b"");
        check("base64", b"f", b"Zg==");
        check("base64", b"fo", b"Zm8=");
        check("base64", b"foo", b"Zm9v");
        check("base64", b"foob", b"Zm9vYg==");
        check("base64", b"fooba", b"Zm9vYmE=");
        check("base64", b"foobar", b"Zm9vYmFy");
    }

    #[test]
    fn encodes_bytes_url64() {
        check("url64", b"", b"");
        check("url64", b"f", b"Zg");
        check("url64", b"fo", b"Zm8");
        check("url64", b"foo", b"Zm9v");
        check("url64", b"foob", b"Zm9vYg");
        check("url64", b"fooba", b"Zm9vYmE");
        check("url64", b"foobar", b"Zm9vYmFy");
    }

    #[test]
    fn default_tests_base64() {
        tests::round_trip("base64");
        tests::basic_configuration("base64");
        tests::invalid_data("base64");
    }

    #[test]
    fn default_tests_url64() {
        tests::round_trip("url64");
        tests::basic_configuration("url64");
        tests::invalid_data("url64");
    }

    #[test]
    fn known_values_base64() {
        check("base64", tests::BYTE_SEQ, b"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/w==");
    }

    #[test]
    fn known_values_url64() {
        check("url64", tests::BYTE_SEQ, b"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0-P0BBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6e3x9fn-AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6ChoqOkpaanqKmqq6ytrq-wsbKztLW2t7i5uru8vb6_wMHCw8TFxsfIycrLzM3Oz9DR0tPU1dbX2Nna29zd3t_g4eLj5OXm5-jp6uvs7e7v8PHy8_T19vf4-fr7_P3-_w");
    }
}
