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
use std::io;

trait Hash {
    fn input(&mut self, data: &[u8]);
    fn result_reset(&mut self) -> Box<[u8]>;
    fn output_size(&self) -> usize;
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
    fn digest(name: &str, length: Option<usize>) -> Result<Box<Hash>, Error> {
        match (name, length) {
            ("adler32", _) => Ok(Box::new(Adler32::new())),
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
        let args: Vec<_> = s
            .args
            .iter()
            .map(|(s, _)| s)
            .filter(|&s| s != "length")
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
        Ok(Encoder::new(Self::digest(args[0], length)?).into_bufread(r, s.bufsize))
    }

    fn options(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("adler32".to_string(), tr!("use Adler32 as the checksum"));
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
        self.digest.input(inp);
        match f {
            FlushState::None => Ok(Status::Ok(inp.len(), 0)),
            FlushState::Finish => {
                let digestlen = self.digest.output_size();
                if outp.len() < digestlen {
                    Ok(Status::BufError(inp.len(), 0))
                } else {
                    outp[0..digestlen].copy_from_slice(&self.digest.result_reset());
                    self.done = true;
                    Ok(Status::StreamEnd(inp.len(), digestlen))
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
    fn default_tests() {
        tests::basic_configuration_without_options("checksum");
    }
}
