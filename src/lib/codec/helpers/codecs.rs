use codec::{Codec, Error, FlushState, Status};
use std::cmp;

pub struct StatelessEncoder<F> {
    f: F,
    bufsize: usize,
}

impl<F> StatelessEncoder<F>
where
    F: Fn(&[u8], &mut [u8]) -> (usize, usize),
{
    pub fn new(f: F, bufsize: usize) -> Self {
        StatelessEncoder { f, bufsize }
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

    fn buffer_size(&self) -> usize {
        self.bufsize
    }
}

pub struct PaddedEncoder<T> {
    enc: T,
    isize: usize,
    osize: usize,
    pad: Option<u8>,
    padfn: fn(usize, usize, usize) -> usize,
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
            padfn: Self::default_padfn,
        }
    }

    pub fn new_with_pad_function(
        enc: T,
        isize: usize,
        osize: usize,
        pad: Option<u8>,
        padfn: fn(usize, usize, usize) -> usize,
    ) -> Self {
        PaddedEncoder {
            enc,
            isize,
            osize,
            pad,
            padfn,
        }
    }

    fn default_padfn(b: usize, is: usize, os: usize) -> usize {
        if b == 0 {
            return 0;
        }
        let bits_per_unit = is * 8;
        let out_bits_per_char = bits_per_unit / os;
        (bits_per_unit - b * 8) / out_bits_per_char
    }

    fn pad_bytes_needed(&self, b: usize) -> usize {
        (self.padfn)(b, self.isize, self.osize)
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
                if b > 0 && padbytes == 0 {
                    return Ok(Status::Ok(a, b));
                }

                let mut inp: Vec<u8> = Vec::new();
                for i in 0..self.isize {
                    let off = a + i;
                    inp.push(if off >= src.len() { 0 } else { src[off] })
                }
                self.enc.transform(inp.as_slice(), &mut dst[b..], f)?;

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

    fn buffer_size(&self) -> usize {
        self.osize
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

    fn offsets(r: Status) -> (usize, usize) {
        match r {
            Status::Ok(a, b) => (a, b),
            Status::SeqError(a, b) => (a, b),
            Status::BufError(a, b) => (a, b),
            Status::StreamEnd(a, b) => (a, b),
        }
    }
}

impl<T: Codec + FilteredDecoder> FilteredDecoder for PaddedDecoder<T> {
    fn strict(&self) -> bool {
        self.codec.strict()
    }

    fn filter_byte(&self, b: u8) -> bool {
        if let Some(pad) = self.pad {
            b == pad || self.codec.filter_byte(b)
        } else {
            self.codec.filter_byte(b)
        }
    }

    fn internal_transform(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        flush: FlushState,
    ) -> Result<Status, Error> {
        match flush {
            FlushState::None if src.len() < self.isize => {
                return Ok(Status::SeqError(0, 0));
            }
            FlushState::Finish if src.is_empty() => {
                return Ok(Status::StreamEnd(0, 0));
            }
            _ => (),
        }

        let r = self.codec.transform(src, dst, flush)?;
        let (a, b) = Self::offsets(r);
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
}

impl<T: Codec + FilteredDecoder> Codec for PaddedDecoder<T> {
    fn transform(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        flush: FlushState,
    ) -> Result<Status, Error> {
        self.wrap_transform(src, dst, flush)
    }

    fn chunk_size(&self) -> usize {
        self.isize
    }

    fn buffer_size(&self) -> usize {
        self.osize
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

impl FilteredDecoder for ChunkedDecoder {
    fn strict(&self) -> bool {
        self.strict
    }

    fn filter_byte(&self, b: u8) -> bool {
        self.table[b as usize] != -1
    }

    fn internal_transform(
        &mut self,
        inp: &[u8],
        outp: &mut [u8],
        f: FlushState,
    ) -> Result<Status, Error> {
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
}

impl Codec for ChunkedDecoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        self.wrap_transform(inp, outp, f)
    }

    fn chunk_size(&self) -> usize {
        self.inpsize
    }

    fn buffer_size(&self) -> usize {
        self.outsize
    }
}

pub struct AffixEncoder<T> {
    codec: T,
    prefix: Vec<u8>,
    suffix: Vec<u8>,
    start: bool,
    end: bool,
}

impl<T: Codec> AffixEncoder<T> {
    pub fn new(codec: T, prefix: Vec<u8>, suffix: Vec<u8>) -> Self {
        AffixEncoder {
            codec,
            prefix,
            suffix,
            start: false,
            end: false,
        }
    }
}

impl<T: Codec> Codec for AffixEncoder<T> {
    fn transform(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        flush: FlushState,
    ) -> Result<Status, Error> {
        if !self.start {
            let prefixlen = self.prefix.len();

            if dst.len() < prefixlen {
                return Ok(Status::BufError(0, 0));
            } else {
                self.start = true;
                dst[0..prefixlen].copy_from_slice(&self.prefix);

                let r = self.transform(src, &mut dst[prefixlen..], flush)?;
                return match r {
                    Status::Ok(a, b) => Ok(Status::Ok(a, b + prefixlen)),
                    Status::SeqError(a, b) => Ok(Status::Ok(a, b + prefixlen)),
                    Status::BufError(a, b) => Ok(Status::Ok(a, b + prefixlen)),
                    Status::StreamEnd(a, b) => Ok(Status::StreamEnd(a, b + prefixlen)),
                };
            }
        }

        if self.end {
            if src.is_empty() {
                return Ok(Status::StreamEnd(0, 0));
            }
            return Err(Error::ExtraData);
        }

        let r = self.codec.transform(src, dst, flush)?;
        match (flush, r, r.unpack()) {
            (FlushState::None, _, _) => Ok(r),
            (FlushState::Finish, _, (a, len)) if len + self.suffix.len() <= dst.len() => {
                let end = len + self.suffix.len();
                dst[len..end].copy_from_slice(&self.suffix);
                self.end = true;
                Ok(Status::StreamEnd(a, end))
            }
            (FlushState::Finish, Status::StreamEnd(a, b), _) => Ok(Status::Ok(a, b)),
            (FlushState::Finish, _, _) => Ok(r),
        }
    }

    fn chunk_size(&self) -> usize {
        self.codec.chunk_size()
    }

    fn buffer_size(&self) -> usize {
        cmp::max(self.prefix.len(), self.suffix.len())
    }
}

/// A trait to help implement non-strict decoding.
///
/// In some codecs, like the `hex` codec, every byte is encoded.  In non-strict mode, we'd want to
/// ignore characters which are not in the encoded alphabet, such as carriage returns, line feeds,
/// and other whitespace.  This trait implementa
pub trait FilteredDecoder {
    /// Returns true if the decoder is in strict mode.
    fn strict(&self) -> bool;
    /// Returns true if the byte is valid in this encoding and false otherwise.
    fn filter_byte(&self, b: u8) -> bool;
    fn filter_sequence(&self, src: &[u8]) -> (Vec<u8>, Vec<usize>) {
        // This Vec maps total filtered input bytes processed to total unfiltered input bytes
        // processed.  Thus, the word "offset" is not the index of the byte in the Vec.
        let mut offsets = Vec::with_capacity(src.len());
        offsets.push(0);
        src.iter()
            .enumerate()
            .filter(|(_i, &b)| self.filter_byte(b))
            .fold(
                (Vec::with_capacity(src.len()), offsets),
                |(mut dest, mut offsets), (i, &b)| {
                    dest.push(b);
                    offsets.push(i + 1);
                    (dest, offsets)
                },
            )
    }
    fn fix_offsets(&self, offsets: &[usize], res: Result<Status, Error>) -> Result<Status, Error> {
        match res {
            Ok(Status::Ok(a, b)) => Ok(Status::Ok(offsets[a], b)),
            Ok(Status::SeqError(a, b)) => Ok(Status::SeqError(offsets[a], b)),
            Ok(Status::BufError(a, b)) => Ok(Status::BufError(offsets[a], b)),
            Ok(Status::StreamEnd(a, b)) => Ok(Status::StreamEnd(offsets[a], b)),
            Err(e) => Err(e),
        }
    }
    /// The main transform.
    ///
    /// This function should process data as if it were operating completely in strict mode.  The
    /// behavior is the same as `Codec::transform`.
    fn internal_transform(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushState,
    ) -> Result<Status, Error>;
    /// The main implementation of the `FilteredDecoder`.
    ///
    /// This function should be called from `Codec::transform` and performs the filtering in
    /// non-strict mode.
    fn wrap_transform(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        flush: FlushState,
    ) -> Result<Status, Error> {
        if self.strict() {
            return self.internal_transform(input, output, flush);
        }
        let (buf, offsets) = self.filter_sequence(input);
        let res = self.internal_transform(&buf, output, flush);
        self.fix_offsets(&offsets, res)
    }
}

#[cfg(test)]
mod tests {
    use super::{AffixEncoder, PaddedDecoder, PaddedEncoder, StatelessEncoder};
    use codec::{Codec, Error, FlushState, Status};
    use std::cmp;

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

        fn buffer_size(&self) -> usize {
            1
        }
    }

    #[derive(Default)]
    pub struct IdentityCodec {}

    impl Codec for IdentityCodec {
        fn transform(
            &mut self,
            inp: &[u8],
            outp: &mut [u8],
            _f: FlushState,
        ) -> Result<Status, Error> {
            let n = cmp::min(inp.len(), outp.len());
            outp[..n].clone_from_slice(&inp[..n]);
            Ok(Status::Ok(n, n))
        }

        fn chunk_size(&self) -> usize {
            1
        }

        fn buffer_size(&self) -> usize {
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
                StatelessEncoder::new(|_, _| (0, 0), osize),
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

    fn affix_encoder() -> AffixEncoder<IdentityCodec> {
        AffixEncoder::new(IdentityCodec::default(), b"<~".to_vec(), b"~>".to_vec())
    }

    #[test]
    fn affix_works_correctly() {
        let tests: &[(&[u8], &[u8])] = &[
            (b"", b"<~~>"),
            (b"abc", b"<~abc~>"),
            (
                b"abcdefghijklmnopqrstuvwxyz",
                b"<~abcdefghijklmnopqrstuvwxyz~>",
            ),
        ];

        for &(inp, outp) in tests {
            // Process as one chunk.
            let mut buf = [0u8; 256];
            let mut codec = affix_encoder();
            let r = codec.transform(inp, &mut buf, FlushState::Finish).unwrap();
            let (r1, r2) = r.unpack();
            assert_eq!(r1, inp.len());
            assert_eq!(&buf[0..r2], outp);

            // Process as lots of tiny chunks.
            let mut buf = [0u8; 256];
            let mut codec = affix_encoder();
            let (mut i, mut j) = (0, 0);
            while i < inp.len() {
                let r = codec
                    .transform(&inp[i..i + 1], &mut buf[j..], FlushState::None)
                    .unwrap();
                let (r1, r2) = r.unpack();
                i += r1;
                j += r2;
            }
            let r = codec
                .transform(&[], &mut buf[j..], FlushState::Finish)
                .unwrap();
            let (r1, r2) = r.unpack();
            i += r1;
            j += r2;
            assert_eq!(i, inp.len());
            assert_eq!(&buf[0..j], outp);
        }
    }
}
