pub mod codecs;
pub mod registry;

#[cfg(test)]
pub mod tests;

use std::cmp;
use std::collections::{BTreeMap, BTreeSet};
use std::convert;
use std::error;
use std::fmt;
use std::io;
use std::io::prelude::*;
use std::ops;

pub const DEFAULT_BUFFER_SIZE: usize = 8192;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    InvalidSequence(String, Vec<u8>),
    TruncatedData,
    ExtraData,
    ForwardOnly(String),
    UnknownCodec(String),
    MissingArgument(String),
    UnknownArgument(String),
    IncompatibleParameters(String, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IO(ref seq) => write!(f, "I/O error: {:?}", seq),
            Error::InvalidSequence(ref name, ref seq) => {
                write!(f, "invalid sequence for codec '{}': {:?}", name, seq)
            }
            Error::TruncatedData => write!(f, "truncated data"),
            Error::ExtraData => write!(f, "extra data"),
            Error::ForwardOnly(ref name) => write!(f, "no reverse transform for {}", name),
            Error::MissingArgument(ref name) => write!(f, "missing argument for {}", name),
            Error::UnknownArgument(ref name) => write!(f, "no such argument: {}", name),
            Error::UnknownCodec(ref name) => write!(f, "no such codec: {}", name),
            Error::IncompatibleParameters(ref name1, ref name2) => write!(
                f,
                "invalid parameter combination: '{}' and '{}",
                name1, name2
            ),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::IO(ref e) => Some(e),
            _ => None,
        }
    }

    // From libstd.
    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }
}

impl convert::From<Error> for io::Error {
    fn from(err: Error) -> Self {
        match err {
            Error::IO(e) => e,
            Error::InvalidSequence(_, _) => io::Error::new(io::ErrorKind::InvalidData, err),
            _ => io::Error::new(io::ErrorKind::InvalidInput, err),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    // We have successfully consumed the given number of valid input bytes and transformed them
    // into valid output bytes. Additional processing may be possible with additional buffer space,
    // however.
    Ok(usize, usize),
    // As above, but the remaining input bytes did not form a complete sequence; we must receive
    // more bytes. If there are none, the sequence has been truncated.
    BufError(usize, usize),
    // As for Ok, but we have detected the end of the input stream and no further bytes should be
    // expected. If there are more bytes, the data is corrupt.
    StreamEnd(usize, usize),
}

impl Status {
    fn unpack(&self) -> (usize, usize) {
        match *self {
            Status::Ok(a, b) => (a, b),
            Status::BufError(a, b) => (a, b),
            Status::StreamEnd(a, b) => (a, b),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FlushState {
    Finish,
    None,
}

pub trait CodecTransform {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error>;
    fn options(&self) -> BTreeMap<String, &'static str>;
    fn can_reverse(&self) -> bool;
    fn name(&self) -> &'static str;
}

pub struct CodecSettings {
    pub bufsize: usize,
    pub strict: bool,
    pub args: BTreeSet<String>,
    pub dir: Direction,
}

pub trait Codec {
    fn transform(&mut self, &[u8], &mut [u8], FlushState) -> Result<Status, Error>;
    fn chunk_size(&self) -> usize;
}

pub trait TransformableCodec<'a, C> {
    fn into_bufread(self, r: Box<io::BufRead>, bufsize: usize) -> Box<io::BufRead + 'a>;
}

impl<'a, C> TransformableCodec<'a, C> for C
where
    C: Codec + 'a,
{
    fn into_bufread(self, r: Box<io::BufRead>, bufsize: usize) -> Box<io::BufRead + 'a> {
        Box::new(Transform::new(r, self, bufsize))
    }
}

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

pub struct CodecReader<R: BufRead, C: Codec> {
    r: R,
    codec: C,
    buf: Vec<u8>,
    off: usize,
}

impl<R: BufRead, C: Codec> CodecReader<R, C> {
    fn new(r: R, c: C, bufsize: usize) -> Self {
        CodecReader {
            r,
            codec: c,
            buf: vec![0u8; bufsize],
            off: 0,
        }
    }

    #[cfg(rustc_1_37)]
    fn memmove<B>(sl: &mut [u8], src: B, dest: usize)
    where
        B: ops::RangeBounds<usize> + IntoIterator<Item = usize>,
    {
        sl.copy_within(src, dest)
    }

    #[cfg(all(not(rustc_1_37), has_range_bounds))]
    fn memmove<B>(sl: &mut [u8], src: B, dest: usize)
    where
        B: ops::RangeBounds<usize> + IntoIterator<Item = usize>,
    {
        match src.start_bound() {
            ops::Bound::Included(&x) if x == dest => return,
            _ => (),
        };
        for (j, i) in src.into_iter().enumerate() {
            sl[j] = sl[i + dest];
        }
    }

    #[cfg(all(not(rustc_1_37), not(has_range_bounds)))]
    fn memmove(sl: &mut [u8], src: ops::Range<usize>, dest: usize) {
        for (j, i) in src.into_iter().enumerate() {
            sl[j] = sl[i + dest];
        }
    }
}

// This function and associated types derived from the flate2 crate.
impl<R: BufRead, C: Codec> Read for CodecReader<R, C> {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        let obj = &mut self.r;
        loop {
            let (ret, eof);
            let last = {
                let read = obj.read(&mut self.buf[self.off..])?;
                let input = &self.buf[..self.off + read];
                eof = read == 0;

                let flush = if eof {
                    FlushState::Finish
                } else {
                    FlushState::None
                };
                ret = { self.codec.transform(input, dst, flush) };
                self.off + read
            };

            match ret {
                Ok(Status::Ok(consumed, _))
                | Ok(Status::BufError(consumed, _))
                | Ok(Status::StreamEnd(consumed, _)) => {
                    Self::memmove(&mut self.buf, consumed..last, 0);
                    self.off = last - consumed;
                }
                _ => (),
            }

            match ret {
                // If we haven't ready any data and we haven't hit EOF yet,
                // then we need to keep asking for more data because if we
                // return that 0 bytes of data have been read then it will
                // be interpreted as EOF.
                Ok(Status::Ok(_, 0)) | Ok(Status::BufError(_, 0)) if !eof && dst.len() > 0 => continue,
                Ok(Status::BufError(0, _)) if eof => {
                    return Err(io::Error::from(Error::TruncatedData))
                }
                Ok(Status::Ok(_, read))
                | Ok(Status::BufError(_, read))
                | Ok(Status::StreamEnd(_, read)) => return Ok(read),

                Err(e) => return Err(io::Error::from(e)),
            }
        }
    }
}

pub struct Transform<C: Codec> {
    b: io::BufReader<CodecReader<Box<io::BufRead>, C>>,
}

impl<C: Codec> Transform<C> {
    pub fn new(r: Box<io::BufRead>, c: C, bufsize: usize) -> Self {
        Transform {
            b: io::BufReader::with_capacity(bufsize, CodecReader::new(r, c, bufsize)),
        }
    }
}

impl<C: Codec> io::BufRead for Transform<C> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.b.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.b.consume(amt)
    }
}

impl<C: Codec> io::Read for Transform<C> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.b.read(buf)
    }
}

pub struct PaddedEncoder<F> {
    enc: StatelessEncoder<F>,
    isize: usize,
    osize: usize,
    pad: Option<u8>,
}

impl<F> PaddedEncoder<F>
where
    F: Fn(&[u8], &mut [u8]) -> (usize, usize),
{
    pub fn new(f: F, isize: usize, osize: usize, pad: Option<u8>) -> Self {
        PaddedEncoder {
            enc: StatelessEncoder::new(f),
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

impl<F> Codec for PaddedEncoder<F>
where
    F: Fn(&[u8], &mut [u8]) -> (usize, usize),
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
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        match f {
            FlushState::None if src.len() < self.isize => {
                return Ok(Status::BufError(0, 0));
            }
            FlushState::Finish if src.len() == 0 => {
                return Ok(Status::StreamEnd(0, 0));
            }
            _ => (),
        }

        let r = self.codec.transform(src, dst, f)?;
        let (a, b) = Self::offsets(r)?;
        // This code relies on us only processing full chunks with the transform function.
        let padoffset = match (self.pad, f) {
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
            iter.map(|(k, &v)| (self.table[(v as usize)] as i64) << ((is - 1 - k) * bits))
                .sum()
        } else {
            iter.filter(|&(_, &x)| self.table[x as usize] != -1)
                .map(|(k, &v)| (self.table[(v as usize)] as i64) << ((is - 1 - k) * bits))
                .sum()
        };

        if x < 0 {
            return Err(Error::InvalidSequence(self.name.to_string(), inp.to_vec()));
        }
        for k in 0..os {
            outp[k] = ((x as u64) >> ((os - 1 - k) * 8) & 0xff) as u8;
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
