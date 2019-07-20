pub mod codecs;
pub mod registry;

#[cfg(test)]
pub mod tests;

use std::collections::BTreeSet;
use std::convert;
use std::error;
use std::fmt;
use std::io;
use std::io::prelude::*;

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    InvalidSequence(String, Vec<u8>),
    TruncatedData,
    ExtraData,
    ForwardOnly(String),
    UnknownCodec(String),
    IncompatibleParameters(String, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IO(ref seq) => write!(f, "I/O error: {:?}", seq),
            Error::InvalidSequence(ref name, ref seq) => {
                write!(f, "invalid sequence for codec '{}': {:02x?}", name, seq)
            }
            Error::TruncatedData => write!(f, "truncated data"),
            Error::ExtraData => write!(f, "extra data"),
            Error::ForwardOnly(ref name) => write!(f, "no reverse transform for {}", name),
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FlushState {
    Finish,
    None,
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
    buf: Buffer,
}

impl<R: BufRead, C: Codec> CodecReader<R, C> {
    fn new(r: R, c: C) -> Self {
        CodecReader {
            r,
            codec: c,
            buf: Buffer::new(),
        }
    }
}

// This function and associated types derived from the flate2 crate.
impl<R: BufRead, C: Codec> Read for CodecReader<R, C> {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        let obj = &mut self.r;
        let mut last_read: Option<usize> = None;
        loop {
            let (ret, eof);
            let (buflen, mut slice) = {
                let buf = obj.fill_buf()?;
                let input = self.buf.slice_for(buf);
                eof = input.is_empty()
                    || match (last_read, input.len()) {
                        (Some(x), y) if x == y => true,
                        _ => false,
                    };

                last_read = Some(input.len());

                let flush = if eof {
                    FlushState::Finish
                } else {
                    FlushState::None
                };
                ret = { self.codec.transform(input.as_ref(), dst, flush) };
                (buf.len(), input)
            };

            match ret {
                Ok(Status::Ok(consumed, _))
                | Ok(Status::BufError(consumed, _))
                | Ok(Status::StreamEnd(consumed, _)) => {
                    slice.consume(consumed);
                    obj.consume(buflen);
                }
                _ => (),
            }

            match ret {
                // If we haven't ready any data and we haven't hit EOF yet,
                // then we need to keep asking for more data because if we
                // return that 0 bytes of data have been read then it will
                // be interpreted as EOF.
                Ok(Status::Ok(_, 0)) | Ok(Status::BufError(_, 0)) if !eof && dst.len() > 0 => {
                    continue
                }
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
    pub fn new(r: Box<io::BufRead>, c: C) -> Self {
        Transform {
            b: io::BufReader::new(CodecReader::new(r, c)),
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
            Ok(Status::Ok(a, b)) => Ok((a, b)),
            Ok(Status::BufError(a, b)) => Ok((a, b)),
            Ok(Status::StreamEnd(a, b)) => Ok((a, b)),
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
            iter.filter(|(_, &x)| self.table[x as usize] != -1)
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
        let n = std::cmp::min(inp.len() / is, outp.len() / os);
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

/// A buffer storing extra bytes in between invocations of a transform.
struct Buffer {
    buf: Vec<u8>,
}
impl Buffer {
    /// Create a new buffer.
    pub fn new() -> Self {
        Buffer { buf: Vec::new() }
    }

    /// Provide a slice consisting of the data from this buffer and `input.
    ///
    /// The slice returned avoids copies if possible.
    pub fn slice_for<'a>(&'a mut self, input: &'a [u8]) -> BufferSlice<'a> {
        if self.buf.len() == 0 {
            BufferSlice::new(&mut self.buf, Some(input), 0)
        } else {
            self.extend(input.iter().cloned());
            BufferSlice::new(&mut self.buf, None, input.len())
        }
    }

    /// Return a slice with this buffer's contents.
    pub fn as_slice(&self) -> &[u8] {
        self.buf.as_slice()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

impl Extend<u8> for Buffer {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = u8>,
    {
        self.buf.extend(iter);
    }
}

impl IntoIterator for Buffer {
    type Item = u8;
    type IntoIter = <Vec<u8> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.buf.into_iter()
    }
}

struct BufferSlice<'a> {
    buf: &'a mut Vec<u8>,
    s: Option<&'a [u8]>,
    consumed: bool,
    size: usize,
}

impl<'a> BufferSlice<'a> {
    fn new(buf: &'a mut Vec<u8>, s: Option<&'a [u8]>, size: usize) -> Self {
        BufferSlice {
            buf,
            s,
            consumed: false,
            size,
        }
    }

    fn len(&self) -> usize {
        self.as_ref().len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Mark the given number of bytes as used and include the remainder into the buffer.
    ///
    /// Returns the number of bytes consumed from the slice `input`. If the number of bytes passed
    /// is smaller than the buffer size, returns zero.
    pub fn consume(&mut self, bytes: usize) -> usize {
        if self.consumed {
            panic!("BufferSlice already consumed");
        }

        self.consumed = true;

        let buflen = self.buf.len() - self.size;
        match (self.s, bytes < buflen) {
            (Some(s), true) => {
                self.buf.drain(0..bytes);
                self.buf.extend(s.as_ref());
                0
            }
            (None, true) => {
                self.buf.drain(0..bytes);
                0
            }
            (Some(s), false) => {
                let consumed = bytes - buflen;
                self.buf.clear();
                self.buf.extend(s.as_ref()[consumed..].iter());
                consumed
            }
            (None, false) => {
                let fullbuflen = self.buf.len();
                self.buf.drain(0..bytes);
                bytes - (fullbuflen - self.size)
            }
        }
    }
}

impl<'a> AsRef<[u8]> for BufferSlice<'a> {
    fn as_ref(&self) -> &[u8] {
        if self.consumed {
            panic!("BufferSlice already consumed");
        }

        match self.s {
            Some(s) => s,
            None => self.buf.as_slice(),
        }
    }
}
