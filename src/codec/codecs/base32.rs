use std::io;
use codec::Codec;
use codec::CodecSettings;
use codec::Direction;
use codec::Error;
use codec::FlushState;
use codec::Status;
use codec::Transform;
use codec::StatelessEncoder;

pub struct TransformFactory {}

pub const BASE32: [u8; 32] = [b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K',
                              b'L', b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T', b'U', b'V',
                              b'W', b'X', b'Y', b'Z', b'2', b'3', b'4', b'5', b'6', b'7'];


pub const BASE32HEX: [u8; 32] = [b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A',
                                 b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L',
                                 b'M', b'N', b'O', b'P', b'Q', b'R', b'S', b'T', b'U', b'V'];
pub const REV: [i8; 256] =
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, 0, -1, -1, -1, 0, 1, 2, 3, 4, 5,
     6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1];

pub const REVHEX: [i8; 256] =
    [-1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -1, -1, 0, -1, -1, -1, 10, 11, 12, 13, 14, 15, 16,
     17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1];

fn forward_transform(inp: &[u8], outp: &mut [u8], arr: &[u8; 32]) -> (usize, usize) {
    let n = std::cmp::min(inp.len() / 5, outp.len() / 8);
    for (i, j) in (0..n).map(|x| (x * 5, x * 8)) {
        let x: u64 = inp[i..i + 5]
            .iter()
            .enumerate()
            .map(|(k, &v)| (v as u64) << ((4 - k) * 8))
            .sum();
        for k in 0..8 {
            outp[j + k] = arr[(x >> ((7 - k) * 5) & 0x1f) as usize];
        }
    }
    (n * 5, n * 8)
}

impl TransformFactory {
    pub fn factory(r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => {
                let enc =
                    PaddedEncoder::new(move |inp, out| forward_transform(inp, out, &BASE32), 5, 8);
                Ok(Box::new(Transform::new(r, enc)))
            }
            Direction::Reverse => {
                Ok(Box::new(Transform::new(r, PaddedDecoder::new(Decoder::new(s.strict), 8, 5))))
            }
        }
    }
}

pub struct PaddedEncoder<F> {
    enc: StatelessEncoder<F>,
    isize: usize,
    osize: usize,
}

impl<F> PaddedEncoder<F>
    where F: Fn(&[u8], &mut [u8]) -> (usize, usize)
{
    pub fn new(f: F, isize: usize, osize: usize) -> Self {
        PaddedEncoder {
            enc: StatelessEncoder::new(f),
            isize: isize,
            osize: osize,
        }
    }

    fn pad_bytes_needed(&self, b: usize) -> usize {
        if b == 0 {
            return 0;
        }
        let bits_per_unit = self.isize * 8;
        let out_bits_per_char = bits_per_unit / self.osize;
        return (bits_per_unit - b * 8) / out_bits_per_char;
    }

    fn offsets(r: Result<Status, Error>) -> Result<(usize, usize), Error> {
        match r {
            Ok(Status::Ok(a, b)) => Ok((a, b)),
            Ok(Status::BufError(a, b)) => Ok((a, b)),
            Ok(Status::StreamEnd(a, b)) => Ok((a, b)),
            Err(e) => Err(e),
        }
    }
}

impl<F> Codec for PaddedEncoder<F>
    where F: Fn(&[u8], &mut [u8]) -> (usize, usize)
{
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let needed = (src.len() + self.isize - 1) / self.isize * self.osize;
        match f {
            FlushState::Finish if src.len() > 0 && dst.len() >= needed => {
                let (a, b) = Self::offsets(self.enc.transform(&src[..src.len() - 1], dst, f))?;
                let padbytes = self.pad_bytes_needed(src.len() - a);
                if padbytes == 0 {
                    return Ok(Status::Ok(a, b));
                }

                let mut inp: Vec<u8> = Vec::new();
                for i in 0..self.isize {
                    let off = a + i;
                    inp.push(if off >= src.len() { 0 } else { src[off] })
                }
                self.enc.transform(inp.as_slice(), dst, f)?;

                let off = self.osize - padbytes;
                for i in &mut dst[b + off..b + self.osize] {
                    *i = '=' as u8;
                }
                Ok(Status::Ok(src.len(), b + self.osize))
            }
            _ => self.enc.transform(src, dst, f),

        }
    }
}

pub struct PaddedDecoder<T> {
    codec: T,
    isize: usize,
    osize: usize,
}

impl<T: Codec> PaddedDecoder<T> {
    pub fn new(codec: T, isize: usize, osize: usize) -> Self {
        PaddedDecoder {
            codec: codec,
            isize: isize,
            osize: osize,
        }
    }

    fn pad_bytes_needed(&self, b: usize) -> usize {
        if b == 0 {
            return 0;
        }
        let bits_per_unit = self.isize * 8;
        let out_bits_per_char = bits_per_unit / self.osize;
        return (bits_per_unit - b * 8) / out_bits_per_char;
    }

    fn offsets(r: Status) -> Result<(usize, usize), Error> {
        match r {
            Status::Ok(a, b) => Ok((a, b)),
            Status::BufError(a, b) => Ok((a, b)),
            Status::StreamEnd(a, b) => Ok((a, b)),
        }
    }
}

impl<T: Codec> Codec for PaddedDecoder<T> {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let r = self.codec.transform(src, dst, f)?;
        let (a, b) = Self::offsets(r)?;
        let padoffset = src.iter().position(|&x| x == b'=');
        let padbytes = match padoffset {
            Some(v) => self.pad_bytes_needed(src.len() - v),
            None => return Ok(r),
        };

        Ok(Status::StreamEnd(a, b - (self.osize - padbytes)))
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
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let n = std::cmp::min(inp.len() / 8, outp.len() / 5);
        for (i, j) in (0..n).map(|x| (x * 8, x * 5)) {
            let iter = inp[i..i + 8].iter().enumerate();
            let x: i64 = if self.strict {
                iter.map(|(k, &v)| (REV[(v as usize)] as i64) << ((7 - k) * 5))
                    .sum()
            } else {
                iter.filter(|(_, &x)| REV[x as usize] != -1)
                    .map(|(k, &v)| (REV[(v as usize)] as i64) << ((7 - k) * 5))
                    .sum()
            };

            if x < 0 {
                return Err(Error::InvalidSequence("base32".to_string(), inp[i..i + 8].to_vec()));
            }
            for k in 0..5 {
                outp[j + k] = ((x as u64) >> ((4 - k) * 8) & 0xff) as u8;
            }
        }

        match f {
            FlushState::Finish if n == inp.len() => Ok(Status::StreamEnd(n + 1, n * 5)),
            _ => Ok(Status::Ok(n * 8, n * 5)),
        }
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::codecs::base32::PaddedEncoder;
    use codec::registry::CodecRegistry;

    #[test]
    fn pads_correctly() {
        let cases = vec![(5, 8, 0, 0),
                         (5, 8, 1, 6),
                         (5, 8, 2, 4),
                         (5, 8, 3, 3),
                         (5, 8, 4, 1),
                         (3, 4, 0, 0),
                         (3, 4, 1, 2),
                         (3, 4, 2, 1)];

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
}
