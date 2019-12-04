#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use std::collections::BTreeMap;
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
    args: BTreeMap<String, Option<String>>,
    dir: Direction,
}

#[derive(Clone)]
pub struct Chain<'a> {
    chain: &'a str,
    bufsize: usize,
    strict: bool,
    codecs: &'a CodecRegistry,
    dir: Direction,
}

impl<'a> Chain<'a> {
    pub fn new(codecs: &'a CodecRegistry, chain: &'a str, bufsize: usize, strict: bool) -> Self {
        Chain {
            chain,
            bufsize,
            strict,
            codecs,
            dir: Direction::Forward,
        }
    }

    pub fn reverse(self) -> Self {
        let mut obj = self.clone();
        obj.dir = obj.dir.invert();
        obj
    }

    pub fn build(&self, src: Box<io::BufRead>) -> io::Result<Box<io::BufRead>> {
        let start: io::Result<_> = Ok(src);
        Self::parse(self.chain, self.dir)?
            .iter()
            .fold(start, |cur, xfrm| {
                Ok(self
                    .codecs
                    .create(xfrm.name, cur?, self.codec_settings(xfrm))?)
            })
    }

    pub fn transform(&self, b: Vec<u8>) -> io::Result<Vec<u8>> {
        let mut out = io::Cursor::new(Vec::new());
        // Cursor provides a BufRead implementation, but we use a BufReader so we can set the
        // buffer size explicitly for test purposes.
        let inp = Box::new(io::BufReader::with_capacity(
            self.bufsize,
            io::Cursor::new(b),
        ));
        let mut res = self.build(inp)?;
        io::copy(&mut res, &mut out)?;
        Ok(out.into_inner())
    }

    fn codec_settings(&self, t: &ChainTransform) -> CodecSettings {
        CodecSettings {
            bufsize: self.bufsize,
            strict: self.strict,
            args: t.args.clone(),
            dir: t.dir,
        }
    }

    fn parse(chain: &str, dir: Direction) -> Result<Vec<ChainTransform>, Error> {
        let iter = chain.split(':').map(|s| Self::parse_unit(s, dir));
        match dir {
            Direction::Forward => iter.collect(),
            Direction::Reverse => iter.rev().collect(),
        }
    }

    fn parse_unit(unit: &str, d: Direction) -> Result<ChainTransform, Error> {
        if unit == "" {
            return Err(Error::InvalidName(String::from(unit)));
        }

        let (s, dir) = if &unit[0..1] == "-" {
            (&unit[1..], Direction::Reverse)
        } else {
            (unit, Direction::Forward)
        };

        let dir = match d {
            Direction::Forward => dir,
            Direction::Reverse => dir.invert(),
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

        let map = match args {
            Some(s) => s
                .split(',')
                .map(Self::parse_arg)
                .collect::<BTreeMap<String, Option<String>>>(),
            None => BTreeMap::new(),
        };
        Ok(ChainTransform {
            name,
            args: map,
            dir,
        })
    }

    fn parse_arg(s: &str) -> (String, Option<String>) {
        match s.find('=') {
            Some(off) => (s[0..off].to_string(), Some(s[off + 1..].to_string())),
            None => (s.to_string(), None),
        }
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use chain::ChainTransform;
    use chain::Error;
    use codec::Direction;

    fn xfrm<'a>(
        s: &'a str,
        v: Vec<(&'a str, Option<&'a str>)>,
        forward: bool,
    ) -> ChainTransform<'a> {
        ChainTransform {
            name: s,
            args: v
                .iter()
                .cloned()
                .map(|(k, v)| (k.to_string(), v.map(|s| s.to_string())))
                .collect(),
            dir: match forward {
                true => Direction::Forward,
                false => Direction::Reverse,
            },
        }
    }

    #[test]
    fn parses_simple_names() {
        assert_eq!(
            Chain::parse("hex", Direction::Forward).unwrap(),
            vec![xfrm("hex", vec![], true)]
        );
        assert_eq!(
            Chain::parse("hex", Direction::Reverse).unwrap(),
            vec![xfrm("hex", vec![], false)]
        );
        assert_eq!(
            Chain::parse("base64", Direction::Forward).unwrap(),
            vec![xfrm("base64", vec![], true)]
        );
        assert_eq!(
            Chain::parse("base64", Direction::Reverse).unwrap(),
            vec![xfrm("base64", vec![], false)]
        );
        assert_eq!(
            Chain::parse("-hex", Direction::Forward).unwrap(),
            vec![xfrm("hex", vec![], false)]
        );
        assert_eq!(
            Chain::parse("-hex", Direction::Reverse).unwrap(),
            vec![xfrm("hex", vec![], true)]
        );
        assert_eq!(
            Chain::parse("-base64", Direction::Forward).unwrap(),
            vec![xfrm("base64", vec![], false)]
        );
        assert_eq!(
            Chain::parse("-base64", Direction::Reverse).unwrap(),
            vec![xfrm("base64", vec![], true)]
        );
    }

    #[test]
    fn parses_parenthesized_names() {
        assert_eq!(
            Chain::parse("hash(sha256)", Direction::Forward).unwrap(),
            vec![xfrm("hash", vec![("sha256", None)], true)]
        );
        assert_eq!(
            Chain::parse("vis(cstyle,white)", Direction::Forward).unwrap(),
            vec![xfrm("vis", vec![("cstyle", None), ("white", None)], true)]
        );
        assert_eq!(
            Chain::parse("-vis(cstyle,white)", Direction::Forward).unwrap(),
            vec![xfrm("vis", vec![("cstyle", None), ("white", None)], false)]
        );
    }

    #[test]
    fn parses_comma_split_names() {
        assert_eq!(
            Chain::parse("hash,sha256", Direction::Forward).unwrap(),
            vec![xfrm("hash", vec![("sha256", None)], true)]
        );
        assert_eq!(
            Chain::parse("vis,cstyle,white", Direction::Forward).unwrap(),
            vec![xfrm("vis", vec![("cstyle", None), ("white", None)], true)]
        );
        assert_eq!(
            Chain::parse("-vis,cstyle,white", Direction::Forward).unwrap(),
            vec![xfrm("vis", vec![("cstyle", None), ("white", None)], false)]
        );
    }

    #[test]
    fn parses_names_with_arguments() {
        assert_eq!(
            Chain::parse("hash(blake2b,length=32)", Direction::Forward).unwrap(),
            vec![xfrm(
                "hash",
                vec![("blake2b", None), ("length", Some("32"))],
                true
            )]
        );
        assert_eq!(
            Chain::parse("hash,blake2b,length=32", Direction::Forward).unwrap(),
            vec![xfrm(
                "hash",
                vec![("blake2b", None), ("length", Some("32"))],
                true
            )]
        );
    }

    #[test]
    fn parses_complex_chains() {
        assert_eq!(
            Chain::parse("-base64:hash,sha256", Direction::Forward).unwrap(),
            vec![
                xfrm("base64", vec![], false),
                xfrm("hash", vec![("sha256", None)], true)
            ]
        );
        assert_eq!(
            Chain::parse("-vis,cstyle,white:xml(html):uri,lower", Direction::Forward).unwrap(),
            vec![
                xfrm("vis", vec![("cstyle", None), ("white", None)], false),
                xfrm("xml", vec![("html", None)], true),
                xfrm("uri", vec![("lower", None)], true)
            ]
        );
        assert_eq!(
            Chain::parse("-vis,cstyle,white:xml(html):uri,lower", Direction::Reverse).unwrap(),
            vec![
                xfrm("uri", vec![("lower", None)], false),
                xfrm("xml", vec![("html", None)], false),
                xfrm("vis", vec![("cstyle", None), ("white", None)], true),
            ]
        );
    }

    #[test]
    fn rejects_invalid_data() {
        assert_eq!(
            Chain::parse("", Direction::Forward).unwrap_err(),
            Error::InvalidName(String::from(""))
        );
        assert_eq!(
            Chain::parse("-", Direction::Forward).unwrap_err(),
            Error::InvalidName(String::from(""))
        );
        assert_eq!(
            Chain::parse("name(", Direction::Forward).unwrap_err(),
            Error::MismatchedParentheses(String::from("name("))
        );
        assert_eq!(
            Chain::parse("-name(", Direction::Forward).unwrap_err(),
            Error::MismatchedParentheses(String::from("name("))
        );
        assert_eq!(
            Chain::parse("hex,", Direction::Forward).unwrap_err(),
            Error::InvalidArgument(String::from(","))
        );
    }
}
