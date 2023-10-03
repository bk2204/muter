#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::Codec;
use codec::CodecSettings;
use codec::CodecTransform;
use codec::Direction;
use codec::Error;
use codec::FlushState;
use codec::Status;
use codec::TransformableCodec;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::io;

trait Hash {
    fn input(&mut self, data: &[u8]);
    fn result_reset(&mut self) -> Box<[u8]>;
    fn input_size(&self) -> usize;
    fn output_size(&self) -> usize;
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum Endianness {
    Little,
    Big,
}

impl Endianness {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "le" => Some(Endianness::Little),
            "be" => Some(Endianness::Big),
            _ => None,
        }
    }

    fn to_u16(self, b: &[u8]) -> u16 {
        match self {
            Endianness::Big => u16::from_be_bytes(b.try_into().unwrap()),
            Endianness::Little => u16::from_le_bytes(b.try_into().unwrap()),
        }
    }
}

struct Adler32 {
    a: u16,
    b: u16,
}

impl Adler32 {
    fn new() -> Adler32 {
        Adler32 { a: 1, b: 0 }
    }
}

impl Hash for Adler32 {
    fn input(&mut self, data: &[u8]) {
        let (a, b) = data.chunks(1024).fold((self.a, self.b), |(a, b), chunk| {
            let (x, y) = chunk
                .iter()
                .fold((a as u32, b as u32), |(mut a, mut b), &x| {
                    a += x as u32;
                    b += a;
                    (a, b)
                });
            ((x % 65521) as u16, (y % 65521) as u16)
        });
        self.a = a;
        self.b = b;
    }

    fn result_reset(&mut self) -> Box<[u8]> {
        let x: u32 = (self.b as u32) << 16 | self.a as u32;
        x.to_be_bytes().to_vec().into_boxed_slice()
    }

    fn input_size(&self) -> usize {
        1
    }

    fn output_size(&self) -> usize {
        4
    }
}

struct Fletcher16 {
    a: u8,
    b: u8,
}

impl Fletcher16 {
    fn new() -> Fletcher16 {
        Fletcher16 { a: 0, b: 0 }
    }
}

impl Hash for Fletcher16 {
    fn input(&mut self, data: &[u8]) {
        let (a, b) = data.chunks(4096).fold((self.a, self.b), |(a, b), chunk| {
            let (x, y) = chunk
                .iter()
                .fold((a as u32, b as u32), |(mut a, mut b), &x| {
                    a += x as u32;
                    b += a;
                    (a, b)
                });
            ((x % 255) as u8, (y % 255) as u8)
        });
        self.a = a;
        self.b = b;
    }

    fn result_reset(&mut self) -> Box<[u8]> {
        let x: u16 = (self.b as u16) << 8 | self.a as u16;
        x.to_be_bytes().to_vec().into_boxed_slice()
    }

    fn input_size(&self) -> usize {
        1
    }

    fn output_size(&self) -> usize {
        2
    }
}

struct Fletcher32 {
    a: u16,
    b: u16,
    endianness: Endianness,
}

impl Fletcher32 {
    fn new(endianness: Endianness) -> Fletcher32 {
        Fletcher32 {
            a: 0,
            b: 0,
            endianness,
        }
    }
}

impl Hash for Fletcher32 {
    fn input(&mut self, data: &[u8]) {
        let (data, overflow): (&[u8], &[u8]) = if data.len() % 2 == 0 {
            (data, b"")
        } else {
            (&data[0..data.len() - 1], &data[data.len() - 1..])
        };
        let (a, b) = data.chunks(720).fold((self.a, self.b), |(a, b), chunk| {
            let (x, y) = chunk
                .chunks(2)
                .fold((a as u32, b as u32), |(mut a, mut b), chunk| {
                    a += self.endianness.to_u16(chunk) as u32;
                    b += a;
                    (a, b)
                });
            ((x % 65535) as u16, (y % 65535) as u16)
        });
        let (a, b) = if overflow.len() == 1 {
            let buf = [overflow[0], 0];
            let (mut a, mut b) = (a as u32, b as u32);
            a += self.endianness.to_u16(&buf) as u32;
            b += a;
            ((a % 65535) as u16, (b % 65535) as u16)
        } else {
            (a, b)
        };
        self.a = a;
        self.b = b;
    }

    fn result_reset(&mut self) -> Box<[u8]> {
        let x: u32 = (self.b as u32) << 16 | self.a as u32;
        x.to_be_bytes().to_vec().into_boxed_slice()
    }

    fn input_size(&self) -> usize {
        2
    }

    fn output_size(&self) -> usize {
        4
    }
}

#[derive(Default)]
pub struct TransformFactory {}

impl TransformFactory {
    pub fn new() -> Self {
        TransformFactory {}
    }
}

impl TransformFactory {
    fn digest(
        name: &str,
        length: Option<usize>,
        endianness: Endianness,
    ) -> Result<Box<Hash>, Error> {
        match (name, length) {
            ("adler32", _) => Ok(Box::new(Adler32::new())),
            ("fletcher16", _) => Ok(Box::new(Fletcher16::new())),
            ("fletcher32", _) => Ok(Box::new(Fletcher32::new(endianness))),
            _ => Err(Error::UnknownArgument(name.to_string())),
        }
    }
}

impl CodecTransform for TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => (),
            Direction::Reverse => return Err(Error::ForwardOnly("checksum".to_string())),
        }

        let length = s.int_arg("length")?;
        let endianness: Vec<_> = s
            .args
            .iter()
            .filter_map(|(s, _)| Endianness::from_str(s))
            .collect();
        let args: Vec<_> = s
            .args
            .keys()
            .filter(|&s| s != "length" && Endianness::from_str(s).is_none())
            .collect();
        match args.len() {
            0 => return Err(Error::MissingArgument("checksum".to_string())),
            1 => (),
            _ => {
                return Err(Error::IncompatibleParameters(
                    args[0].to_string(),
                    args[1].to_string(),
                ));
            }
        };
        let endianness = match endianness.len() {
            0 => Endianness::Big,
            1 => endianness[0],
            _ => {
                return Err(Error::IncompatibleParameters(
                    "be".to_string(),
                    "le".to_string(),
                ));
            }
        };
        Ok(Encoder::new(Self::digest(args[0], length, endianness)?).into_bufread(r, s.bufsize))
    }

    fn options(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("adler32".to_string(), tr!("use Adler32 as the checksum"));
        map.insert(
            "fletcher16".to_string(),
            tr!("use Fletcher16 as the checksum"),
        );
        map.insert(
            "fletcher32".to_string(),
            tr!("use Fletcher32 as the checksum"),
        );
        map
    }

    fn can_reverse(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "checksum"
    }
}

pub struct Encoder {
    digest: Box<Hash>,
    done: bool,
}

impl Codec for Encoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        if self.done {
            return Ok(Status::StreamEnd(0, 0));
        }
        let read = match f {
            FlushState::None => {
                let last = inp.len() - (inp.len() % self.digest.input_size());
                self.digest.input(&inp[0..last]);
                last
            }
            FlushState::Finish => {
                self.digest.input(inp);
                inp.len()
            }
        };
        match f {
            FlushState::None => Ok(Status::Ok(read, 0)),
            FlushState::Finish => {
                let digestlen = self.digest.output_size();
                if outp.len() < digestlen {
                    Ok(Status::BufError(read, 0))
                } else {
                    outp[0..digestlen].copy_from_slice(&self.digest.result_reset());
                    self.done = true;
                    Ok(Status::StreamEnd(read, digestlen))
                }
            }
        }
    }

    fn chunk_size(&self) -> usize {
        1
    }

    fn buffer_size(&self) -> usize {
        self.digest.output_size()
    }
}

impl Encoder {
    fn new(digest: Box<Hash>) -> Self {
        Encoder {
            digest,
            done: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(algo: &str, inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        let codec = format!("checksum({}):hex", algo);
        let dlen = outp.len() / 2;
        for i in vec![dlen, dlen + 1, dlen + 2, dlen + 3, 512] {
            let c = Chain::new(&reg, &codec, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, &codec, i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
        }
    }

    #[test]
    fn adler32() {
        // Test vectors from https://github.com/froydnj/ironclad/blob/master/testing/test-vectors/adler32.testvec.
        check("adler32", b"", b"00000001");
        check("adler32", b"a", b"00620062");
        check("adler32", b"abc", b"024d0127");
        check("adler32", b"message digest", b"29750586");
        check("adler32", b"abcdefghijklmnopqrstuvwxyz", b"90860b20");
        check(
            "adler32",
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789",
            b"8adb150c",
        );
        check(
            "adler32",
            b"12345678901234567890123456789012345678901234567890123456789012345678901234567890",
            b"97b61069",
        );
    }

    #[test]
    fn fletcher16() {
        // Test vectors from Wikipedia.
        check("fletcher16", b"", b"0000");
        check("fletcher16", b"abcde", b"c8f0");
        check("fletcher16", b"abcdef", b"2057");
        check("fletcher16", b"abcdefgh", b"0627");
    }

    #[test]
    fn fletcher32() {
        // Test vectors from Wikipedia.
        check("fletcher32,le", b"", b"00000000");
        check("fletcher32,le", b"abcde", b"f04fc729");
        check("fletcher32,le", b"abcdef", b"56502d2a");
        check("fletcher32,le", b"abcdefgh", b"ebe19591");
        // Generated with a small Ruby program.
        check("fletcher32,be", b"", b"00000000");
        check("fletcher32,be", b"abcde", b"4ff029c7");
        check("fletcher32,be", b"abcdef", b"50562a2d");
        check("fletcher32,be", b"abcdefg", b"e183912d");
        check("fletcher32,be", b"abcdefgh", b"e1eb9195");
    }

    #[test]
    fn default_tests() {
        tests::basic_configuration_without_options("checksum");
    }
}
