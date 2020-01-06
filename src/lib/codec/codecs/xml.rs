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
use std::char;
use std::collections::BTreeMap;
use std::io;
use std::str;

enum Characters {
    LessThan = 0,
    GreaterThan = 1,
    Ampersand = 2,
    Apostrophe = 3,
    QuotationMark = 4,
}

// The maximum length in bytes of any encoding.
const MAX_LEN: usize = 5;

const DEFAULT: [&[u8]; 5] = [b"&lt;", b"&gt;", b"&amp;", b"&apos;", b"&quot;"];

const HTML: [&[u8]; 5] = [b"&lt;", b"&gt;", b"&amp;", b"&#x27;", b"&quot;"];

const HEX: [&[u8]; 5] = [b"&#x3c;", b"&#x3e;", b"&#x26;", b"&#x27;", b"&#x22;"];

fn copy_to(x: Characters, arr: &[&[u8]; 5], outp: &mut [u8]) -> usize {
    let r = arr[x as usize];
    outp[0..r.len()].copy_from_slice(r);
    r.len()
}

fn forward_transform(inp: &[u8], outp: &mut [u8], arr: &[&[u8]; 5]) -> (usize, usize) {
    let max = outp.len() - MAX_LEN;
    let maxin = inp.len();
    let (mut i, mut j) = (0, 0);
    while i < maxin && j < max {
        let x = inp[i];
        j += match x {
            b'<' => copy_to(Characters::LessThan, arr, &mut outp[j..]),
            b'>' => copy_to(Characters::GreaterThan, arr, &mut outp[j..]),
            b'&' => copy_to(Characters::Ampersand, arr, &mut outp[j..]),
            b'\'' => copy_to(Characters::Apostrophe, arr, &mut outp[j..]),
            b'"' => copy_to(Characters::QuotationMark, arr, &mut outp[j..]),
            _ => {
                outp[j] = x;
                1
            }
        };
        i += 1;
    }
    (i, j)
}

#[derive(Default)]
pub struct XMLTransformFactory {}

impl XMLTransformFactory {
    pub fn new() -> Self {
        XMLTransformFactory {}
    }
}

impl CodecTransform for XMLTransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => {
                let arr = if s.args.contains_key("hex") {
                    HEX
                } else if s.args.contains_key("html") {
                    HTML
                } else {
                    DEFAULT
                };
                Ok(
                    StatelessEncoder::new(move |inp, out| forward_transform(inp, out, &arr))
                        .into_bufread(r, s.bufsize),
                )
            }
            Direction::Reverse => Ok(Decoder::new().into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        let mut map = BTreeMap::new();
        map.insert("default".to_string(), "use XML entity names");
        map.insert(
            "hex".to_string(),
            "use hexadecimal entity names for XML entities",
        );
        map.insert(
            "html".to_string(),
            "use HTML-friendly entity names for XML entities",
        );
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "xml"
    }
}

pub struct Decoder {}

impl Decoder {
    fn new() -> Self {
        Decoder {}
    }
}

macro_rules! unpack {
    ($val:expr, $err:expr) => {
        match $val {
            Ok(x) => x,
            Err(_) => return $err,
        };
    };
}

impl Decoder {
    fn process_char(src: &[u8], dst: &mut [u8], radix: u32) -> Result<usize, Error> {
        let err = Err(Error::InvalidSequence("xml".to_string(), src.to_vec()));
        let s = unpack!(str::from_utf8(src), err);
        let val = unpack!(u32::from_str_radix(s, radix), err);
        let ch: char = match char::from_u32(val) {
            Some(c) => c,
            None => return err,
        };
        Ok(ch.encode_utf8(dst).len())
    }
}

impl Codec for Decoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let mut iter = src.iter().enumerate();
        let mut j = 0;
        while let Some((i, x)) = iter.next() {
            // Is there space for another UTF-8 character?
            if j >= dst.len() - 4 {
                return Ok(Status::Ok(i, j));
            }

            match *x {
                b'&' => {
                    let name: Vec<_> = iter
                        .by_ref()
                        .take_while(|&(_, &x)| x != b';')
                        .map(|(_, &x)| x)
                        .collect();
                    // If we reached the end of the string without finding a semicolon, then
                    // either we need more data or we've got a truncated sequence.
                    if src.len() == name.len() + i + 1 {
                        match f {
                            FlushState::None => return Ok(Status::SeqError(i, j)),
                            FlushState::Finish => return Err(Error::TruncatedData),
                        }
                    }
                    if name.len() < 2 {
                        return Err(Error::InvalidSequence("xml".to_string(), name));
                    }
                    j += match (name[0], name[1]) {
                        (b'#', b'x') | (b'#', b'X') => {
                            if name.len() < 3 {
                                return Err(Error::InvalidSequence("xml".to_string(), name));
                            }
                            Self::process_char(&name[2..], &mut dst[j..], 16)?
                        }
                        (b'#', _) => Self::process_char(&name[1..], &mut dst[j..], 10)?,
                        _ => {
                            match name.as_slice() {
                                b"lt" => dst[j] = b'<',
                                b"gt" => dst[j] = b'>',
                                b"apos" => dst[j] = b'\'',
                                b"quot" => dst[j] = b'"',
                                b"amp" => dst[j] = b'&',
                                _ => return Err(Error::InvalidSequence("xml".to_string(), name)),
                            };
                            1
                        }
                    };
                }
                _ => {
                    dst[j] = *x;
                    j += 1;
                }
            }
        }
        Ok(Status::Ok(src.len(), j))
    }

    fn chunk_size(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;
    use codec::Error;

    fn check(name: &str, inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![10, 11, 12, 512] {
            let c = Chain::new(&reg, name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, "-xml", i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-xml", i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    fn check_decode(inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![10, 11, 12, 512] {
            let c = Chain::new(&reg, "-xml", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, "-xml", i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
        }
    }

    macro_rules! check_failure {
        ($inp:expr, $x:pat) => {
            let reg = CodecRegistry::new();
            for i in vec![6, 7, 8, 512] {
                for b in vec![true, false] {
                    let c = Chain::new(&reg, "-xml", i, b);
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
        check("xml", b"abc", b"abc");
        check("xml", br#"<>&'""#, b"&lt;&gt;&amp;&apos;&quot;");
        check("xml,default", br#"<>&'""#, b"&lt;&gt;&amp;&apos;&quot;");
        check("xml,hex", br#"<>&'""#, b"&#x3c;&#x3e;&#x26;&#x27;&#x22;");
        check("xml,html", br#"<>&'""#, b"&lt;&gt;&amp;&#x27;&quot;");
        check("xml", b"&#x10ffff;", b"&amp;#x10ffff;");
    }

    #[test]
    fn decodes_bytes() {
        check_decode(b"&#48;", b"0");
        check_decode(b"&#x30;", b"0");
        check_decode(b"&#xfeff;", b"\xef\xbb\xbf");
        check_decode(b"&#65279;", b"\xef\xbb\xbf");
        check_decode(b"&#x10ffff;", b"\xf4\x8f\xbf\xbf");
        check_decode(b"&#1114111;", b"\xf4\x8f\xbf\xbf");
        check_decode(b"&#1114111;&#x10fffe;", b"\xf4\x8f\xbf\xbf\xf4\x8f\xbf\xbe");
    }

    #[test]
    fn default_tests() {
        tests::round_trip("xml");
        tests::round_trip("xml,default");
        tests::round_trip("xml,hex");
        tests::round_trip("xml,html");
        tests::basic_configuration("xml");
        tests::invalid_data("xml");
    }

    #[test]
    fn rejects_invalid() {
        check_failure!(b"&abc;", Error::InvalidSequence(_, _));
        check_failure!(b"&l;", Error::InvalidSequence(_, _));
        check_failure!(b"&lt", Error::TruncatedData);
        check_failure!(b"&#x", Error::TruncatedData);
        check_failure!(b"&#x2", Error::TruncatedData);
        check_failure!(b"&#2", Error::TruncatedData);
    }
}
