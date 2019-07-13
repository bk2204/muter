pub mod codecs;
pub mod registry;

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
}

pub struct StatelessEncoder<F> {
    f: F,
}

impl<F> StatelessEncoder<F>
where
    F: Fn(&[u8], &mut [u8]) -> (usize, usize),
{
    pub fn new(f: F) -> Self {
        StatelessEncoder { f: f }
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
}

pub struct CodecReader<R: BufRead, C: Codec> {
    r: R,
    codec: C,
}

impl<R: BufRead, C: Codec> CodecReader<R, C> {
    fn new(r: R, c: C) -> Self {
        CodecReader { r: r, codec: c }
    }
}

// This function and associated types derived from the flate2 crate.
impl<R: BufRead, C: Codec> Read for CodecReader<R, C> {
    fn read(&mut self, dst: &mut [u8]) -> io::Result<usize> {
        let obj = &mut self.r;
        let mut last_read: Option<usize> = None;
        loop {
            let (ret, eof);
            {
                let input = obj.fill_buf()?;
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
                ret = { self.codec.transform(input, dst, flush) };
            }

            match ret {
                Ok(Status::Ok(consumed, _))
                | Ok(Status::BufError(consumed, _))
                | Ok(Status::StreamEnd(consumed, _)) => obj.consume(consumed),
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
