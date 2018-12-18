use std::io;
use codec::Codec;
use codec::CodecSettings;
use codec::Direction;
use codec::Error;
use codec::FlushState;
use codec::Status;
use codec::Transform;
use codec::StatelessEncoder;

use codec::codecs::hex::{LOWER, UPPER, REV};

pub struct TransformFactory {}

fn forward_transform(inp: &[u8], outp: &mut [u8], arr: &[u8; 16]) -> (usize, usize) {
    let max = outp.len() - 3;
    let maxin = inp.len();
    let (mut i, mut j) = (0, 0);
    while i < maxin && j < max {
        let x = inp[i];
        match x {
            b'-' | b'.' | b'0'...b'9' | b'A'...b'Z' | b'_' | b'a'...b'z' | b'~' => outp[j] = x,
            0x00...0x2c | 0x2f | 0x3a...0x40 | 0x5b...0x5e | 0x60 | 0x7b...0x7d | 0x7f...0xff => {
                outp[j + 0] = b'%';
                outp[j + 1] = arr[(x as usize) >> 4];
                outp[j + 2] = arr[(x as usize) & 15];
                j += 2;
            }
            _ => unreachable!(),
        }

        i += 1;
        j += 1;
    }
    (i, j)
}

impl TransformFactory {
    pub fn factory(r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => {
                let arr = match s.args.contains("lower") {
                    true => &LOWER,
                    false => &UPPER,
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
    fn transform(&mut self, src: &[u8], dst: &mut [u8], _f: FlushState) -> Result<Status, Error> {
        let mut iter = src.iter().enumerate();
        let mut j = 0;
        loop {
            let s = iter.next();
            let (i, x) = match s {
                Some((a, b)) => (a, b),
                None => break,
            };
            if j == dst.len() {
                return Ok(Status::Ok(i, j));
            }

            match x {
                b'-' | b'.' | b'0'...b'9' | b'A'...b'Z' | b'_' | b'a'...b'z' | b'~' => dst[j] = *x,
                b'%' => {
                    let y = iter.next();
                    let z = iter.next();
                    match (y, z) {
                        (Some((_, a)), Some((_, b))) => {
                            let v: i16 = ((REV[*a as usize] as i16) << 4) | REV[*b as usize] as i16;
                            if v < 0 {
                                return Err(Error::InvalidSequence("uri".to_string(), vec![*a, *b]));
                            }
                            dst[j] = v as u8;
                        }
                        _ => return Ok(Status::BufError(i, j)),
                    }
                }
                _ => {
                    if self.strict {
                        return Err(Error::InvalidSequence("uri".to_string(), vec![*x]));
                    }
                    dst[j] = *x;
                }
            }
            j += 1;
        }
        return Ok(Status::Ok(src.len(), j));
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::Error;
    use codec::registry::CodecRegistry;

    fn reg() -> CodecRegistry {
        CodecRegistry::new()
    }

    fn check(inp: &[u8], lower: &[u8], upper: &[u8]) {
        for i in vec![4, 5, 6, 7, 8, 512] {
            let c = Chain::new(reg(), "uri", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
            let c = Chain::new(reg(), "uri,lower", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), lower);
            let c = Chain::new(reg(), "uri,upper", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
            let c = Chain::new(reg(), "-uri", i, true);
            assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
            let c = Chain::new(reg(), "-uri", i, true);
            assert_eq!(c.transform(lower.to_vec()).unwrap(), inp);
            let c = Chain::new(reg(), "-uri", i, false);
            assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
        }
    }

    macro_rules! check_failure {
        ($inp:expr, $x:pat) => {
            for i in vec![4, 5, 6, 7, 8, 512] {
                for b in vec![true, false] {
                    let c = Chain::new(reg(), "-uri", i, b);
                    match c.transform($inp.to_vec()) {
                        Ok(_) => panic!("got success for invalid sequence"),
                        Err(e) => {
                            match e.get_ref().unwrap().downcast_ref::<Error>() {
                                Some($x) => (),
                                Some(e) => panic!("got wrong error: {:?}", e),
                                None => panic!("No internal error?"),
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn encodes_bytes() {
        check(b"abc", b"abc", b"abc");
        check(b"\x00\xff", b"%00%ff", b"%00%FF");
        check(b"\xc2\xa9", b"%c2%a9", b"%C2%A9");
        check(b"\x01\x23\x45\x67\x89\xab\xcd\xef",
              b"%01%23Eg%89%ab%cd%ef",
              b"%01%23Eg%89%AB%CD%EF");
        check(b"\xfe\xdc\xba", b"%fe%dc%ba", b"%FE%DC%BA");
    }

    #[test]
    fn rejects_invalid() {
        check_failure!(b"abc%0xff", Error::InvalidSequence(_, _));
        check_failure!(b"abc%", Error::TruncatedData);
        check_failure!(b"abc%v", Error::TruncatedData);
        check_failure!(b"abc%vv", Error::InvalidSequence(_, _));
    }
}
