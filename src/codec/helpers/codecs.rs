use codec::{Codec, Error, FlushState, Status};
use std::cmp;

pub struct StatelessEncoder<F> {
    f: F,
}

impl<F> StatelessEncoder<F>
where
    F: Fn(&[u8], &mut [u8]) -> (usize, usize),
{
    pub fn new(f: F) -> Self {
        StatelessEncoder { f }
    }
}

impl<F> Codec for StatelessEncoder<F>
where
    F: Fn(&[u8], &mut [u8]) -> (usize, usize),
{
    fn transform(&mut self, inp: &[u8], out: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let consumed = (self.f)(inp, out);
        match f {
            FlushState::Finish if consumed.0 == inp.len() => {
                Ok(Status::StreamEnd(consumed.0, consumed.1))
            }
            _ => Ok(Status::Ok(consumed.0, consumed.1)),
        }
    }

    fn chunk_size(&self) -> usize {
        1
    }
}

pub struct PaddedEncoder<T> {
    enc: T,
    isize: usize,
    osize: usize,
    pad: Option<u8>,
}

impl<T> PaddedEncoder<T>
where
    T: Codec,
{
    pub fn new(enc: T, isize: usize, osize: usize, pad: Option<u8>) -> Self {
        PaddedEncoder {
            enc,
            isize,
            osize,
            pad,
        }
    }

    fn pad_bytes_needed(&self, b: usize) -> usize {
        if b == 0 {
            return 0;
        }
        let bits_per_unit = self.isize * 8;
        let out_bits_per_char = bits_per_unit / self.osize;
        (bits_per_unit - b * 8) / out_bits_per_char
    }

    fn offsets(r: Result<Status, Error>) -> Result<(usize, usize), Error> {
        match r {
            Ok(s) => Ok(s.unpack()),
            Err(e) => Err(e),
        }
    }
}

impl<T> Codec for PaddedEncoder<T>
where
    T: Codec,
{
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let needed = (src.len() + self.isize - 1) / self.isize * self.osize;
        match f {
            FlushState::Finish if !src.is_empty() && dst.len() >= needed => {
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
                match self.pad {
                    Some(byte) => {
                        for i in &mut dst[b + off..b + self.osize] {
                            *i = byte;
                        }
                        Ok(Status::Ok(src.len(), b + self.osize))
                    }
                    None => Ok(Status::Ok(src.len(), b + off)),
                }
            }
            _ => self.enc.transform(src, dst, f),
        }
    }

    fn chunk_size(&self) -> usize {
        self.isize
    }
}

pub struct PaddedDecoder<T> {
    codec: T,
    isize: usize,
    osize: usize,
    pad: Option<u8>,
}

impl<T: Codec> PaddedDecoder<T> {
    pub fn new(codec: T, isize: usize, osize: usize, pad: Option<u8>) -> Self {
        PaddedDecoder {
            codec,
            isize,
            osize,
            pad,
        }
    }

    /// Returns the number of bytes to trim off a complete unit, given a number of input bytes
    /// excluding pad bytes.
    fn bytes_to_trim(&self, x: usize) -> usize {
        let b = x % self.isize;
        if b == 0 {
            return 0;
        }
        let b = self.isize - b;
        let bits_per_unit = self.osize * 8;
        let in_bits_per_char = bits_per_unit / self.isize;
        self.osize - (bits_per_unit - b * in_bits_per_char) / 8
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
    fn transform(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        flush: FlushState,
    ) -> Result<Status, Error> {
        match flush {
            FlushState::None if src.len() < self.isize => {
                return Ok(Status::BufError(0, 0));
            }
            FlushState::Finish if src.is_empty() => {
                return Ok(Status::StreamEnd(0, 0));
            }
            _ => (),
        }

        let r = self.codec.transform(src, dst, flush)?;
        let (a, b) = Self::offsets(r)?;
        // This code relies on us only processing full chunks with the transform function.
        let padoffset = match (self.pad, flush) {
            (Some(byte), _) => src[..a].iter().position(|&x| x == byte),
            (None, FlushState::Finish) if a == src.len() => Some(src.len()),
            (None, _) => None,
        };
        let trimbytes = match padoffset {
            Some(v) => self.bytes_to_trim(v),
            None => return Ok(r),
        };

        Ok(Status::Ok(a, b - trimbytes))
    }

    fn chunk_size(&self) -> usize {
        self.isize
    }
}

pub struct ChunkedDecoder {
    strict: bool,
    inpsize: usize,
    outsize: usize,
    name: &'static str,
    table: &'static [i8; 256],
}

impl ChunkedDecoder {
    pub fn new(
        strict: bool,
        name: &'static str,
        inpsize: usize,
        outsize: usize,
        table: &'static [i8; 256],
    ) -> Self {
        ChunkedDecoder {
            strict,
            inpsize,
            outsize,
            name,
            table,
        }
    }
}

impl ChunkedDecoder {
    fn process_chunk(&self, inp: &[u8], outp: &mut [u8]) -> Result<Status, Error> {
        let (is, os, bits) = (self.inpsize, self.outsize, self.outsize * 8 / self.inpsize);
        let iter = inp.iter().enumerate();
        let x: i64 = if self.strict {
            iter.map(|(k, &v)| i64::from(self.table[(v as usize)]) << ((is - 1 - k) * bits))
                .sum()
        } else {
            iter.filter(|&(_, &x)| self.table[x as usize] != -1)
                .map(|(k, &v)| i64::from(self.table[(v as usize)]) << ((is - 1 - k) * bits))
                .sum()
        };

        if x < 0 {
            return Err(Error::InvalidSequence(self.name.to_string(), inp.to_vec()));
        }
        for (k, val) in outp.iter_mut().enumerate().take(os) {
            *val = ((x as u64) >> ((os - 1 - k) * 8) & 0xff) as u8;
        }
        Ok(Status::Ok(inp.len(), os))
    }
}

impl Codec for ChunkedDecoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let (is, os) = (self.inpsize, self.outsize);
        let n = cmp::min(inp.len() / is, outp.len() / os);
        for (i, j) in (0..n).map(|x| (x * is, x * os)) {
            self.process_chunk(&inp[i..i + is], &mut outp[j..j + os])?;
        }

        match f {
            FlushState::Finish if n == inp.len() => Ok(Status::StreamEnd(n, n * os)),
            FlushState::Finish if inp.len() < is && outp.len() >= os => {
                self.process_chunk(inp, &mut outp[0..os])?;
                Ok(Status::StreamEnd(inp.len(), os))
            }
            _ => Ok(Status::Ok(n * is, n * os)),
        }
    }

    fn chunk_size(&self) -> usize {
        self.inpsize
    }
}

#[cfg(test)]
mod tests {
    use super::{PaddedDecoder, PaddedEncoder, StatelessEncoder};
    use codec::{Codec, Error, FlushState, Status};

    // Test objects.
    pub struct TestCodec {}

    impl TestCodec {
        pub fn new() -> Self {
            TestCodec {}
        }
    }

    impl Codec for TestCodec {
        fn transform(
            &mut self,
            inp: &[u8],
            outp: &mut [u8],
            _f: FlushState,
        ) -> Result<Status, Error> {
            Ok(Status::StreamEnd(inp.len(), outp.len()))
        }

        fn chunk_size(&self) -> usize {
            1
        }
    }

    // Tests.
    #[test]
    fn pads_encoding_correctly() {
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
            let p = PaddedEncoder::new(
                StatelessEncoder::new(|_, _| (0, 0)),
                isize,
                osize,
                Some(b'='),
            );
            assert_eq!(p.pad_bytes_needed(inbytes), padbytes);
        }
    }

    #[test]
    fn pads_decoding_correctly() {
        let cases = vec![
            (5, 8, 0, 0),
            (5, 8, 2, 4),
            (5, 8, 4, 3),
            (5, 8, 5, 2),
            (5, 8, 7, 1),
            (3, 4, 0, 0),
            (3, 4, 2, 2),
            (3, 4, 3, 1),
            (3, 4, 10, 2),
        ];

        for (isize, osize, inbytes, padbytes) in cases {
            let p = PaddedDecoder::new(TestCodec::new(), osize, isize, Some(b'='));
            assert_eq!(p.bytes_to_trim(inbytes), padbytes);
        }
    }
}
