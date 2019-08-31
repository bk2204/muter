#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use std::collections::BTreeSet;
use std::convert;
use std::error;
use std::fmt;
use std::io;

use codec::registry::CodecRegistry;
use codec::CodecSettings;
use codec::Direction;

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    MismatchedParentheses(String),
    InvalidName(String),
    InvalidArgument(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::MismatchedParentheses(ref seq) => write!(f, "mismatched parentheses: {:?}", seq),
            Error::InvalidName(ref seq) => write!(f, "invalid transform name: {:?}", seq),
            Error::InvalidArgument(ref seq) => write!(f, "invalid argument: {:?}", seq),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match *self {
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
        io::Error::new(io::ErrorKind::InvalidInput, err)
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ChainTransform<'a> {
    name: &'a str,
    args: BTreeSet<String>,
    dir: Direction,
}

pub struct Chain<'a> {
    chain: &'a str,
    bufsize: usize,
    strict: bool,
    codecs: &'a CodecRegistry,
}

impl<'a> Chain<'a> {
    pub fn new(codecs: &'a CodecRegistry, chain: &'a str, bufsize: usize, strict: bool) -> Self {
        Chain {
            chain,
            bufsize,
            strict,
            codecs,
        }
    }

    pub fn build(&self, src: Box<io::BufRead>) -> io::Result<Box<io::BufRead>> {
        let start: io::Result<_> = Ok(src);
        Self::parse(self.chain)?.iter().fold(start, |cur, xfrm| {
            Ok(self
                .codecs
                .create(xfrm.name, cur?, self.codec_settings(xfrm))?)
        })
    }

    pub fn transform<'b>(&self, b: Vec<u8>) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        {
            let mut out = io::Cursor::new(&mut buf);
            // Cursor provides a BufRead implementation, but we use a BufReader so we can set the
            // buffer size explicitly for test purposes.
            let inp = Box::new(io::BufReader::with_capacity(
                self.bufsize,
                io::Cursor::new(b),
            ));
            let mut res = self.build(inp)?;
            io::copy(&mut res, &mut out)?;
        }
        Ok(buf)
    }

    fn codec_settings(&self, t: &ChainTransform) -> CodecSettings {
        CodecSettings {
            bufsize: self.bufsize,
            strict: self.strict,
            args: t.args.clone(),
            dir: t.dir.clone(),
        }
    }

    fn parse(chain: &str) -> Result<Vec<ChainTransform>, Error> {
        chain.split(':').map(|s| Self::parse_unit(s)).collect()
    }

    fn parse_unit(unit: &str) -> Result<ChainTransform, Error> {
        if unit == "" {
            return Err(Error::InvalidName(String::from(unit)));
        }

        let (s, dir) = if &unit[0..1] == "-" {
            (&unit[1..], Direction::Reverse)
        } else {
            (unit, Direction::Forward)
        };

        let (name, args): (&str, Option<&str>) = if let Some(off) = s.find('(') {
            if &s[s.len() - 1..] != ")" {
                return Err(Error::MismatchedParentheses(String::from(s)));
            }
            (&s[0..off], Some(&s[off + 1..s.len() - 1]))
        } else {
            match s.find(',') {
                Some(off) if off != s.len() - 1 => (&s[0..off], Some(&s[off + 1..])),
                Some(off) => return Err(Error::InvalidArgument(String::from(&s[off..]))),
                None => (s, None),
            }
        };

        if name == "" {
            return Err(Error::InvalidName(String::from(name)));
        }

        let set = match args {
            Some(s) => s
                .split(',')
                .map(|x| String::from(x))
                .collect::<BTreeSet<String>>(),
            None => BTreeSet::new(),
        };
        Ok(ChainTransform {
            name,
            args: set,
            dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::iter::FromIterator;

    use chain::Chain;
    use chain::ChainTransform;
    use chain::Error;
    use codec::Direction;

    fn xfrm<'a>(s: &'a str, v: Vec<&'a str>, forward: bool) -> ChainTransform<'a> {
        ChainTransform {
            name: s,
            args: BTreeSet::from_iter(v.iter().map(|&x| String::from(x))),
            dir: match forward {
                true => Direction::Forward,
                false => Direction::Reverse,
            },
        }
    }

    #[test]
    fn parses_simple_names() {
        assert_eq!(
            Chain::parse("hex").unwrap(),
            vec![xfrm("hex", vec![], true)]
        );
        assert_eq!(
            Chain::parse("base64").unwrap(),
            vec![xfrm("base64", vec![], true)]
        );
        assert_eq!(
            Chain::parse("-hex").unwrap(),
            vec![xfrm("hex", vec![], false)]
        );
        assert_eq!(
            Chain::parse("-base64").unwrap(),
            vec![xfrm("base64", vec![], false)]
        );
    }

    #[test]
    fn parses_parenthesized_names() {
        assert_eq!(
            Chain::parse("hash(sha256)").unwrap(),
            vec![xfrm("hash", vec!["sha256"], true)]
        );
        assert_eq!(
            Chain::parse("vis(cstyle,white)").unwrap(),
            vec![xfrm("vis", vec!["cstyle", "white"], true)]
        );
        assert_eq!(
            Chain::parse("-vis(cstyle,white)").unwrap(),
            vec![xfrm("vis", vec!["cstyle", "white"], false)]
        );
    }

    #[test]
    fn parses_comma_split_names() {
        assert_eq!(
            Chain::parse("hash,sha256").unwrap(),
            vec![xfrm("hash", vec!["sha256"], true)]
        );
        assert_eq!(
            Chain::parse("vis,cstyle,white").unwrap(),
            vec![xfrm("vis", vec!["cstyle", "white"], true)]
        );
        assert_eq!(
            Chain::parse("-vis,cstyle,white").unwrap(),
            vec![xfrm("vis", vec!["cstyle", "white"], false)]
        );
    }

    #[test]
    fn parses_complex_chains() {
        assert_eq!(
            Chain::parse("-base64:hash,sha256").unwrap(),
            vec![
                xfrm("base64", vec![], false),
                xfrm("hash", vec!["sha256"], true)
            ]
        );
        assert_eq!(
            Chain::parse("-vis,cstyle,white:xml(html):uri,lower").unwrap(),
            vec![
                xfrm("vis", vec!["cstyle", "white"], false),
                xfrm("xml", vec!["html"], true),
                xfrm("uri", vec!["lower"], true)
            ]
        );
    }

    #[test]
    fn rejects_invalid_data() {
        assert_eq!(
            Chain::parse("").unwrap_err(),
            Error::InvalidName(String::from(""))
        );
        assert_eq!(
            Chain::parse("-").unwrap_err(),
            Error::InvalidName(String::from(""))
        );
        assert_eq!(
            Chain::parse("name(").unwrap_err(),
            Error::MismatchedParentheses(String::from("name("))
        );
        assert_eq!(
            Chain::parse("-name(").unwrap_err(),
            Error::MismatchedParentheses(String::from("name("))
        );
        assert_eq!(
            Chain::parse("hex,").unwrap_err(),
            Error::InvalidArgument(String::from(","))
        );
    }
}
