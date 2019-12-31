pub mod codecs;
pub mod helpers;
pub mod registry;
pub mod tests;

use std;
use std::collections::BTreeMap;
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
    InvalidArgument(String, String),
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
            Error::InvalidArgument(ref name, ref val) => {
                write!(f, "value for argument {} is invalid: {}", name, val)
            }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Forward,
    Reverse,
}

impl Direction {
    pub fn invert(self) -> Self {
        match self {
            Direction::Forward => Direction::Reverse,
            Direction::Reverse => Direction::Forward,
        }
    }
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
    pub args: BTreeMap<String, Option<String>>,
    pub dir: Direction,
}

impl CodecSettings {
    fn int_arg<T: std::str::FromStr + ToString + Ord + From<u8> + Copy>(
        &self,
        name: &str,
    ) -> Result<Option<T>, Error> {
        match self.args.get(name) {
            Some(&None) => Err(Error::MissingArgument(name.to_string())),
            Some(&Some(ref val)) => match val.parse() {
                Ok(x) => Ok(Some(x)),
                Err(_) => Err(Error::InvalidArgument(name.to_string(), val.clone())),
            },
            None => Ok(None),
        }
    }

    fn length_arg<T: std::str::FromStr + ToString + Ord + From<u8> + Copy>(
        &self,
        name: &str,
        min: T,
        default: Option<T>,
    ) -> Result<Option<T>, Error> {
        let zero: T = 0u8.into();
        let val: Option<T> = match self.int_arg(name)? {
            Some(x) if x == zero => None,
            Some(x) if x >= min => Some(x),
            Some(x) => return Err(Error::InvalidArgument(name.to_string(), x.to_string())),
            None => default,
        };
        Ok(val)
    }

    fn bool_arg(&self, name: &str) -> Result<bool, Error> {
        match self.args.get(name) {
            Some(&None) => Ok(true),
            Some(&Some(ref val)) => Err(Error::InvalidArgument(name.to_string(), val.clone())),
            None => Ok(false),
        }
    }
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
                Ok(Status::Ok(_, 0)) | Ok(Status::BufError(_, 0)) if !eof && !dst.is_empty() => {
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
