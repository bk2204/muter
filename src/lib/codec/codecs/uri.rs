#![allow(unknown_lints)]
#![allow(bare_trait_objects)]
#![allow(ellipsis_inclusive_range_patterns)]

use codec::helpers::codecs::StatelessEncoder;
use codec::Codec;
use codec::CodecSettings;
use codec::CodecTransform;
use codec::Direction;
use codec::Error;
use codec::FlushState;
use codec::Status;
use codec::TransformableCodec;
use std::collections::BTreeMap;
use std::io;

use codec::codecs::hex::{LOWER, REV, UPPER};

enum Characters {
    Identity,
    Space,
    Encoded,
}

// Identity: b'-' | b'.' | b'0'...b'9' | b'A'...b'Z' | b'_' | b'a'...b'z' | b'~'
// Space: b' '
// Encoded: everything else
const TABLE: [Characters; 256] = [
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Space,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Identity,
    Characters::Identity,
    Characters::Encoded,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Identity,
    Characters::Encoded,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Identity,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
];

fn forward_transform(
    inp: &[u8],
    outp: &mut [u8],
    arr: &[u8; 16],
    special_plus: bool,
) -> (usize, usize) {
    let max = outp.len() - 3;
    let maxin = inp.len();
    let (mut i, mut j) = (0, 0);
    while i < maxin && j < max {
        let x = inp[i];
        match TABLE[x as usize] {
            Characters::Identity => outp[j] = x,
            Characters::Space if special_plus => outp[j] = b'+',
            _ => {
                outp[j..j + 3].copy_from_slice(&[
                    b'%',
                    arr[(x as usize) >> 4],
                    arr[(x as usize) & 15],
                ]);
                j += 2;
            }
        }

        i += 1;
        j += 1;
    }
    (i, j)
}

#[derive(Default)]
pub struct URITransformFactory {}

impl URITransformFactory {
    pub fn new() -> Self {
        URITransformFactory {}
    }
}

impl CodecTransform for URITransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        Self::generic_factory(r, s, false)
    }

    fn options(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("lower".to_string(), "use lowercase letters".to_string());
        map.insert("upper".to_string(), "use uppercase letters".to_string());
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "uri"
    }
}

impl URITransformFactory {
    fn generic_factory(
        r: Box<io::BufRead>,
        s: CodecSettings,
        form: bool,
    ) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => {
                let arr = if s.bool_arg("lower")? { &LOWER } else { &UPPER };
                Ok(
                    StatelessEncoder::new(
                        move |inp, out| forward_transform(inp, out, arr, form),
                        3,
                    )
                    .into_bufread(r, s.bufsize),
                )
            }
            Direction::Reverse => Ok(Decoder::new(s.strict, form).into_bufread(r, s.bufsize)),
        }
    }
}

#[derive(Default)]
pub struct FormTransformFactory {}

impl FormTransformFactory {
    pub fn new() -> Self {
        FormTransformFactory {}
    }
}

impl CodecTransform for FormTransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        URITransformFactory::generic_factory(r, s, true)
    }

    fn options(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("lower".to_string(), "use lowercase letters".to_string());
        map.insert("upper".to_string(), "use uppercase letters".to_string());
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "form"
    }
}

pub struct Decoder {
    strict: bool,
    special_plus: bool,
}

impl Decoder {
    fn new(strict: bool, special_plus: bool) -> Self {
        Decoder {
            strict,
            special_plus,
        }
    }
}

impl Codec for Decoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], _f: FlushState) -> Result<Status, Error> {
        let mut iter = src.iter().enumerate();
        let mut j = 0;
        while let Some((i, x)) = iter.next() {
            if j == dst.len() {
                return Ok(Status::Ok(i, j));
            }

            match *x {
                b'-' | b'.' | b'0'...b'9' | b'A'...b'Z' | b'_' | b'a'...b'z' | b'~' => dst[j] = *x,
                b'%' => {
                    let b1 = iter.next();
                    let b2 = iter.next();
                    match (b1, b2) {
                        (Some((_, c1)), Some((_, c2))) => {
                            let val: i16 =
                                (i16::from(REV[*c1 as usize]) << 4) | i16::from(REV[*c2 as usize]);
                            if val < 0 {
                                return Err(Error::InvalidSequence(
                                    "uri".to_string(),
                                    vec![*c1, *c2],
                                ));
                            }
                            dst[j] = val as u8;
                        }
                        _ => return Ok(Status::SeqError(i, j)),
                    }
                }
                b'+' if self.special_plus => dst[j] = b' ',
                _ => {
                    if self.strict {
                        return Err(Error::InvalidSequence("uri".to_string(), vec![*x]));
                    }
                    dst[j] = *x;
                }
            }
            j += 1;
        }
        Ok(Status::Ok(src.len(), j))
    }

    fn chunk_size(&self) -> usize {
        3
    }

    fn buffer_size(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;
    use codec::Error;

    fn check_uri(inp: &[u8], lower: &[u8], upper: &[u8]) {
        check("uri", inp, lower, upper);
    }

    fn check_form(inp: &[u8], lower: &[u8], upper: &[u8]) {
        check("form", inp, lower, upper);
    }

    fn check_all(inp: &[u8], lower: &[u8], upper: &[u8]) {
        check_uri(inp, lower, upper);
        check_form(inp, lower, upper);
    }

    fn check(name: &str, inp: &[u8], lower: &[u8], upper: &[u8]) {
        let reg = CodecRegistry::new();
        let lname = format!("{},lower", name);
        let uname = format!("{},upper", name);
        let reverse = format!("-{}", name);
        for i in vec![4, 5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
            let c = Chain::new(&reg, &lname, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), lower);
            let c = Chain::new(&reg, &uname, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
            let c = Chain::new(&reg, &reverse, i, true);
            assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, &reverse, i, true);
            assert_eq!(c.transform(lower.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, &reverse, i, false);
            assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
        }
    }

    macro_rules! check_failure {
        ($rev:expr, $inp:expr, $x:pat) => {
            let reg = CodecRegistry::new();
            for i in vec![4, 5, 6, 7, 8, 512] {
                for b in vec![true, false] {
                    let c = Chain::new(&reg, $rev, i, b);
                    match c.transform($inp.to_vec()) {
                        Ok(_) => panic!("got success for invalid sequence"),
                        Err(e) => match e.get_ref().unwrap().downcast_ref::<Error>() {
                            Some(&$x) => (),
                            Some(e) => panic!("got wrong error: {:?}", e),
                            None => panic!("No internal error?"),
                        },
                    }
                }
            }
        };
    }

    #[test]
    fn encodes_bytes() {
        check_all(b"abc", b"abc", b"abc");
        check_all(b"abc", b"abc", b"abc");
        check_all(b"\x00\xff", b"%00%ff", b"%00%FF");
        check_all(b"\xc2\xa9", b"%c2%a9", b"%C2%A9");
        check_all(
            b"\x01\x23\x45\x67\x89\xab\xcd\xef",
            b"%01%23Eg%89%ab%cd%ef",
            b"%01%23Eg%89%AB%CD%EF",
        );
        check_all(b"\xfe\xdc\xba", b"%fe%dc%ba", b"%FE%DC%BA");
    }

    #[test]
    fn encodes_bytes_uri() {
        check_uri(b"a b", b"a%20b", b"a%20b");
    }

    #[test]
    fn encodes_bytes_form() {
        check_form(b"a b", b"a+b", b"a+b");
    }

    fn rejects_invalid(rev: &str) {
        check_failure!(rev, b"abc%0xff", Error::InvalidSequence(_, _));
        check_failure!(rev, b"abc%", Error::TruncatedData);
        check_failure!(rev, b"abc%v", Error::TruncatedData);
        check_failure!(rev, b"abc%vv", Error::InvalidSequence(_, _));
    }

    #[test]
    fn rejects_invalid_uri() {
        rejects_invalid("-uri");
    }

    #[test]
    fn rejects_invalid_form() {
        rejects_invalid("-form");
    }

    #[test]
    fn round_trip_uri() {
        tests::round_trip("uri");
        tests::round_trip("uri,upper");
        tests::round_trip("uri,lower");
        tests::basic_configuration("uri");
        tests::invalid_data("uri");
    }

    #[test]
    fn round_trip_form() {
        tests::round_trip("form");
        tests::round_trip("form,upper");
        tests::round_trip("form,lower");
        tests::basic_configuration("form");
        tests::invalid_data("form");
    }

    #[test]
    fn known_values_uri() {
        check_uri(tests::BYTE_SEQ, b"%00%01%02%03%04%05%06%07%08%09%0a%0b%0c%0d%0e%0f%10%11%12%13%14%15%16%17%18%19%1a%1b%1c%1d%1e%1f%20%21%22%23%24%25%26%27%28%29%2a%2b%2c-.%2f0123456789%3a%3b%3c%3d%3e%3f%40ABCDEFGHIJKLMNOPQRSTUVWXYZ%5b%5c%5d%5e_%60abcdefghijklmnopqrstuvwxyz%7b%7c%7d~%7f%80%81%82%83%84%85%86%87%88%89%8a%8b%8c%8d%8e%8f%90%91%92%93%94%95%96%97%98%99%9a%9b%9c%9d%9e%9f%a0%a1%a2%a3%a4%a5%a6%a7%a8%a9%aa%ab%ac%ad%ae%af%b0%b1%b2%b3%b4%b5%b6%b7%b8%b9%ba%bb%bc%bd%be%bf%c0%c1%c2%c3%c4%c5%c6%c7%c8%c9%ca%cb%cc%cd%ce%cf%d0%d1%d2%d3%d4%d5%d6%d7%d8%d9%da%db%dc%dd%de%df%e0%e1%e2%e3%e4%e5%e6%e7%e8%e9%ea%eb%ec%ed%ee%ef%f0%f1%f2%f3%f4%f5%f6%f7%f8%f9%fa%fb%fc%fd%fe%ff", b"%00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F%10%11%12%13%14%15%16%17%18%19%1A%1B%1C%1D%1E%1F%20%21%22%23%24%25%26%27%28%29%2A%2B%2C-.%2F0123456789%3A%3B%3C%3D%3E%3F%40ABCDEFGHIJKLMNOPQRSTUVWXYZ%5B%5C%5D%5E_%60abcdefghijklmnopqrstuvwxyz%7B%7C%7D~%7F%80%81%82%83%84%85%86%87%88%89%8A%8B%8C%8D%8E%8F%90%91%92%93%94%95%96%97%98%99%9A%9B%9C%9D%9E%9F%A0%A1%A2%A3%A4%A5%A6%A7%A8%A9%AA%AB%AC%AD%AE%AF%B0%B1%B2%B3%B4%B5%B6%B7%B8%B9%BA%BB%BC%BD%BE%BF%C0%C1%C2%C3%C4%C5%C6%C7%C8%C9%CA%CB%CC%CD%CE%CF%D0%D1%D2%D3%D4%D5%D6%D7%D8%D9%DA%DB%DC%DD%DE%DF%E0%E1%E2%E3%E4%E5%E6%E7%E8%E9%EA%EB%EC%ED%EE%EF%F0%F1%F2%F3%F4%F5%F6%F7%F8%F9%FA%FB%FC%FD%FE%FF");
    }

    #[test]
    fn known_values_form() {
        check_form(tests::BYTE_SEQ, b"%00%01%02%03%04%05%06%07%08%09%0a%0b%0c%0d%0e%0f%10%11%12%13%14%15%16%17%18%19%1a%1b%1c%1d%1e%1f+%21%22%23%24%25%26%27%28%29%2a%2b%2c-.%2f0123456789%3a%3b%3c%3d%3e%3f%40ABCDEFGHIJKLMNOPQRSTUVWXYZ%5b%5c%5d%5e_%60abcdefghijklmnopqrstuvwxyz%7b%7c%7d~%7f%80%81%82%83%84%85%86%87%88%89%8a%8b%8c%8d%8e%8f%90%91%92%93%94%95%96%97%98%99%9a%9b%9c%9d%9e%9f%a0%a1%a2%a3%a4%a5%a6%a7%a8%a9%aa%ab%ac%ad%ae%af%b0%b1%b2%b3%b4%b5%b6%b7%b8%b9%ba%bb%bc%bd%be%bf%c0%c1%c2%c3%c4%c5%c6%c7%c8%c9%ca%cb%cc%cd%ce%cf%d0%d1%d2%d3%d4%d5%d6%d7%d8%d9%da%db%dc%dd%de%df%e0%e1%e2%e3%e4%e5%e6%e7%e8%e9%ea%eb%ec%ed%ee%ef%f0%f1%f2%f3%f4%f5%f6%f7%f8%f9%fa%fb%fc%fd%fe%ff", b"%00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F%10%11%12%13%14%15%16%17%18%19%1A%1B%1C%1D%1E%1F+%21%22%23%24%25%26%27%28%29%2A%2B%2C-.%2F0123456789%3A%3B%3C%3D%3E%3F%40ABCDEFGHIJKLMNOPQRSTUVWXYZ%5B%5C%5D%5E_%60abcdefghijklmnopqrstuvwxyz%7B%7C%7D~%7F%80%81%82%83%84%85%86%87%88%89%8A%8B%8C%8D%8E%8F%90%91%92%93%94%95%96%97%98%99%9A%9B%9C%9D%9E%9F%A0%A1%A2%A3%A4%A5%A6%A7%A8%A9%AA%AB%AC%AD%AE%AF%B0%B1%B2%B3%B4%B5%B6%B7%B8%B9%BA%BB%BC%BD%BE%BF%C0%C1%C2%C3%C4%C5%C6%C7%C8%C9%CA%CB%CC%CD%CE%CF%D0%D1%D2%D3%D4%D5%D6%D7%D8%D9%DA%DB%DC%DD%DE%DF%E0%E1%E2%E3%E4%E5%E6%E7%E8%E9%EA%EB%EC%ED%EE%EF%F0%F1%F2%F3%F4%F5%F6%F7%F8%F9%FA%FB%FC%FD%FE%FF");
    }
}
