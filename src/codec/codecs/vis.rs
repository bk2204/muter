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
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::iter::Peekable;

#[derive(PartialEq, Eq, Debug)]
enum Character {
    Identity,
    Octal,
    Glob,
    Control,
    Meta,
    ControlMeta,
    Space,
    Bell,
    Backspace,
    FormFeed,
    Newline,
    CarriageReturn,
    Tab,
    VerticalTab,
    Nul,
    Backslash,
    MetaSpace,
}

const TABLE: [Character; 256] = [
    Character::Nul,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Bell,
    Character::Backspace,
    Character::Tab,
    Character::Newline,
    Character::VerticalTab,
    Character::FormFeed,
    Character::CarriageReturn,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Control,
    Character::Space,
    Character::Identity,
    Character::Identity,
    Character::Glob,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Glob,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Octal,
    Character::Octal,
    Character::Octal,
    Character::Octal,
    Character::Octal,
    Character::Octal,
    Character::Octal,
    Character::Octal,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Glob,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Glob,
    Character::Backslash,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Identity,
    Character::Control,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::ControlMeta,
    Character::MetaSpace,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::Meta,
    Character::ControlMeta,
];

#[derive(Default)]
pub struct VisTransformFactory {}

impl VisTransformFactory {
    pub fn new() -> Self {
        VisTransformFactory {}
    }
}

impl CodecTransform for VisTransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => Ok(Encoder::new(&s.args).into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(Decoder::new().into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        let mut map = BTreeMap::new();
        map.insert("cstyle".to_string(), "encode using C-like escape sequences");
        map.insert(
            "glob".to_string(),
            "encode characters recognized by glob(3) and hash mark",
        );
        map.insert("nl".to_string(), "encode newline");
        map.insert("octal".to_string(), "encode using octal escape sequences");
        map.insert("sp".to_string(), "encode space");
        map.insert("space".to_string(), "encode space");
        map.insert("tab".to_string(), "encode tab");
        map.insert("white".to_string(), "encode space, tab, and newline");
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "vis"
    }
}

pub struct Encoder {
    table: Vec<Vec<u8>>,
    cstyle: bool,
}

impl Codec for Encoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        assert!(self.table.len() == 256);

        let max = outp.len() - 4;
        let maxin = inp.len();
        let (mut i, mut j) = (0, 0);
        while i < maxin && j < max {
            let x = inp[i];
            // If this is a NUL byte and we're in cstyle-mode,…
            let s: &[u8] = if x == b'\0' && self.cstyle {
                // …and there's another character which is an octal character, then write this out
                // as a full three-digit escape.
                if i + 1 < maxin && TABLE[inp[i + 1] as usize] == Character::Octal {
                    b"\\000"
                } else if i + 1 == maxin && f == FlushState::None {
                    return Ok(Status::BufError(i, j));
                } else {
                    b"\\0"
                }
            } else {
                self.table[x as usize].as_slice()
            };

            outp[j..j + s.len()].copy_from_slice(s);

            i += 1;
            j += s.len();
        }
        Ok(Status::Ok(i, j))
    }

    fn chunk_size(&self) -> usize {
        1
    }
}

impl Encoder {
    pub fn new(args: &BTreeSet<String>) -> Self {
        Encoder {
            table: Self::build_table(args),
            cstyle: args.contains("cstyle"),
        }
    }

    fn build_table(args: &BTreeSet<String>) -> Vec<Vec<u8>> {
        let cstyle = args.contains("cstyle");
        let octal = args.contains("octal");
        let glob = args.contains("glob");
        let space = args.contains("sp") || args.contains("space") || args.contains("white");
        let tab = args.contains("tab") || args.contains("white");
        let nl = args.contains("nl") || args.contains("white");

        (0..256)
            .map(|x| {
                let i = x as u8;
                match TABLE[i as usize] {
                    Character::Glob if glob => Self::encode(i, true),
                    Character::Bell if cstyle => b"\\a".to_vec(),
                    Character::Backspace if cstyle => b"\\b".to_vec(),
                    Character::FormFeed if cstyle => b"\\f".to_vec(),
                    Character::Newline if nl && cstyle => b"\\n".to_vec(),
                    Character::Newline if nl => Self::encode(i, octal),
                    Character::Space if space && cstyle => b"\\s".to_vec(),
                    Character::Space if space => Self::encode(i, octal),
                    Character::Tab if tab && cstyle => b"\\t".to_vec(),
                    Character::Tab if tab => Self::encode(i, octal),
                    Character::CarriageReturn if cstyle => b"\\r".to_vec(),
                    Character::VerticalTab if cstyle => b"\\v".to_vec(),
                    Character::Nul if cstyle => b"\\0".to_vec(),
                    Character::Backslash => b"\\\\".to_vec(),
                    Character::Bell
                    | Character::Backspace
                    | Character::FormFeed
                    | Character::CarriageReturn
                    | Character::VerticalTab
                    | Character::Nul
                    | Character::MetaSpace
                    | Character::Control
                    | Character::Meta
                    | Character::ControlMeta => Self::encode(i, octal),
                    _ => vec![i],
                }
            })
            .collect()
    }

    fn encode(c: u8, octal: bool) -> Vec<u8> {
        if octal || c == 0x20 || c == 0xa0 {
            vec![
                b'\\',
                (c >> 6) + b'0',
                ((c >> 3) & 7) + b'0',
                (c & 7) + b'0',
            ]
        } else if c < 0x80 {
            vec![b'\\', b'^', c ^ 0x40]
        } else if c < 0xa0 || c == 0xff {
            vec![b'\\', b'M', b'^', c ^ 0xc0]
        } else {
            vec![b'\\', b'M', b'-', c ^ 0x80]
        }
    }
}

pub struct Decoder {}

impl Decoder {
    fn new() -> Self {
        Decoder {}
    }
}

impl Decoder {
    /// Processes an escape, starting from the character after the backslash.
    ///
    /// `offset` is the index of the backslash in the original array.
    fn handle_escape<'a, I: Iterator<Item = (usize, &'a u8)>>(
        &self,
        offset: usize,
        iter: &mut Peekable<I>,
        dst: &mut [u8],
        f: FlushState,
    ) -> Result<Status, Error> {
        // This function returns different values than the typical transform method.  A typical
        // method returns the number of bytes consumed in each of the source and destination
        // arrays.  Here, we return the number of bytes consumed from `transform`'s input on the
        // read side (the former item in the array) but the number of bytes consumed from _our_
        // destination array on the write side (the latter item).
        //
        // Note that these are the number of bytes consumed, not the offset of the byte consumed.
        // On success, the number of bytes consumed will be one more than the offset of the last
        // byte.
        assert!(dst.len() >= 2);

        // This is the value we return indicating we need more data.
        let moredata = Status::BufError(offset, 0);
        let x = iter.next();
        match x {
            // \^C
            Some((_, &b'^')) => match iter.next() {
                Some((i, y)) => {
                    dst[0] = y ^ 0x40;
                    Ok(Status::Ok(i + 1, 1))
                }
                None => Ok(moredata),
            },
            // \M-C and \M^C
            Some((i, &b'M')) => {
                let y = iter.next();
                let z = iter.next();
                match (y, z) {
                    (Some((_, &b'-')), Some((_, c))) => dst[0] = c | 0x80,
                    (Some((_, &b'-')), None) => return Ok(moredata),
                    (Some((_, &b'^')), Some((_, c))) => dst[0] = c ^ 0xc0,
                    (Some((_, &b'^')), None) => return Ok(moredata),
                    (Some((_, &c1)), _) => {
                        return Err(Error::InvalidSequence(
                            "vis".to_string(),
                            vec![b'\\', b'M', c1],
                        ))
                    }
                    (None, _) => return Ok(moredata),
                };
                Ok(Status::Ok(i + 3, 1))
            }
            Some((i, &b'0')) => {
                let y = iter.peek().cloned();
                match (y, f) {
                    // \0
                    (None, FlushState::Finish) => {
                        dst[0] = b'\0';
                        Ok(Status::Ok(i + 1, 1))
                    }
                    (None, FlushState::None) => Ok(moredata),
                    // 3-digit octal escape
                    (Some((_, &c)), _) if c >= b'0' && c <= b'7' => {
                        iter.next();
                        match iter.next() {
                            Some((j, &c2)) => {
                                dst[0] = ((c - b'0') << 3) | (c2 - b'0');
                                Ok(Status::Ok(j + 1, 1))
                            }
                            None => Ok(moredata),
                        }
                    }
                    // \0 followed by a new escape
                    (Some((_, &b'\\')), _) => {
                        dst[0] = b'\0';
                        Ok(Status::Ok(i + 1, 1))
                    }
                    // \0 followed by normal character
                    (Some((_, _)), _) => {
                        let (i, &c1) = iter.next().unwrap();
                        dst[0] = b'\0';
                        dst[1] = c1;
                        Ok(Status::Ok(i + 1, 2))
                    }
                }
            }
            // 3-digit octal escape
            Some((_, &c1)) if c1 >= b'1' && c1 <= b'7' => {
                let y = iter.next();
                let z = iter.next();
                match (y, z) {
                    (None, _) | (_, None) => Ok(moredata),
                    (Some((_, &c2)), Some((i, &c3))) => {
                        dst[0] = ((c1 - b'0') << 6) | ((c2 - b'0') << 3) | (c3 - b'0');
                        Ok(Status::Ok(i + 1, 1))
                    }
                }
            }
            // \\
            Some((i, &b'\\')) => {
                dst[0] = b'\\';
                Ok(Status::Ok(i + 1, 1))
            }
            Some((i, &c)) => {
                dst[0] = match c {
                    b'a' => b'\x07',
                    b'b' => b'\x08',
                    b'f' => b'\x0c',
                    b'n' => b'\n',
                    b'r' => b'\r',
                    b's' => b' ',
                    b't' => b'\t',
                    b'v' => b'\x0b',
                    _ => return Err(Error::InvalidSequence("vis".to_string(), vec![b'\\', c])),
                };
                Ok(Status::Ok(i + 1, 1))
            }
            None => Ok(moredata),
        }
    }
}

impl Codec for Decoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let mut iter = src.iter().enumerate().peekable();
        let mut j = 0;
        loop {
            let s = iter.next();
            let (i, x) = match s {
                Some((a, b)) => (a, b),
                None => break,
            };
            if j >= dst.len() - 1 {
                return Ok(Status::Ok(i, j));
            }

            match *x {
                b'\\' => match self.handle_escape(i, &mut iter, &mut dst[j..], f)? {
                    Status::Ok(_, b) => j += b,
                    Status::BufError(a, b) => return Ok(Status::BufError(a, j + b)),
                    Status::StreamEnd(a, b) => return Ok(Status::StreamEnd(a, j + b)),
                },
                _ => {
                    dst[j] = *x;
                    j += 1;
                }
            }
        }
        Ok(Status::Ok(src.len(), j))
    }

    fn chunk_size(&self) -> usize {
        4
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;
    use codec::Error;

    fn check(options: &[&'static str], inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        let optstring = options.join(",");
        let name = format!("vis{}{}", if optstring == "" { "" } else { "," }, optstring);
        for i in vec![4, 5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, &name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, "-vis", i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-vis", i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    macro_rules! check_failure {
        ($inp:expr, $x:pat) => {
            let reg = CodecRegistry::new();
            for i in vec![4, 5, 6, 7, 8, 512] {
                for b in vec![true, false] {
                    let c = Chain::new(&reg, "-vis", i, b);
                    match c.transform($inp.to_vec()) {
                        Ok(_) => panic!("got success for invalid sequence"),
                        Err(e) => match e.get_ref().unwrap().downcast_ref::<Error>() {
                            Some(&$x) => (),
                            Some(e) => panic!("got wrong error: {:?}", e),
                            None => panic!("No internal error?"),
                        },
                    }
                }
            }
        };
    }

    #[test]
    fn encodes_bytes() {
        check(&[], b"abc", b"abc");
        check(&["octal"], b"\x00\xff", b"\\000\\377");
        check(&["octal", "cstyle"], b"\x00\x01\xff", b"\\0\\001\\377");
        check(&[], b"\x00\xfe", b"\\^@\\M-~");
        check(&[], b"\x00\xff", b"\\^@\\M^?");
        check(&["cstyle"], b"^^\0", b"^^\\0");
        check(&["cstyle"], b"\0/", b"\\0/");
        check(&["cstyle"], b"\00", b"\\0000");
        check(&["cstyle"], b"\01", b"\\0001");
        check(&["cstyle"], b"\07", b"\\0007");
        check(&["cstyle"], b"\08", b"\\08");
        check(
            &["cstyle"],
            b"\x0e\xa4\xa4\x00\x31\x6b",
            b"\\^N\\M-$\\M-$\\0001k",
        );
        check(&["cstyle"], b"\\\\0", b"\\\\\\\\0");
    }

    #[test]
    fn rejects_invalid() {
        check_failure!(b"abc\\x", Error::InvalidSequence(_, _));
        check_failure!(b"abc\\", Error::TruncatedData);
        check_failure!(b"abc\\^", Error::TruncatedData);
        check_failure!(b"abc\\M", Error::TruncatedData);
        check_failure!(b"abc\\M-", Error::TruncatedData);
        check_failure!(b"abc\\M^", Error::TruncatedData);
        check_failure!(b"abc\\Mx", Error::InvalidSequence(_, _));
    }

    #[test]
    fn round_trip() {
        tests::round_trip("vis");
        tests::round_trip("vis,cstyle");
        tests::round_trip("vis,octal");
        tests::round_trip("vis,cstyle,octal");
        tests::round_trip("vis,cstyle,space,glob");
        tests::round_trip("vis,octal,space,glob");
        tests::round_trip("vis,cstyle,octal,space,glob");
    }

    #[test]
    fn known_values() {
        check(&[], tests::BYTE_SEQ, b"\\^@\\^A\\^B\\^C\\^D\\^E\\^F\\^G\\^H\t\n\\^K\\^L\\^M\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_ !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["sp"], tests::BYTE_SEQ, b"\\^@\\^A\\^B\\^C\\^D\\^E\\^F\\^G\\^H\t\n\\^K\\^L\\^M\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_\\040!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["space"], tests::BYTE_SEQ, b"\\^@\\^A\\^B\\^C\\^D\\^E\\^F\\^G\\^H\t\n\\^K\\^L\\^M\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_\\040!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["tab"], tests::BYTE_SEQ, b"\\^@\\^A\\^B\\^C\\^D\\^E\\^F\\^G\\^H\\^I\n\\^K\\^L\\^M\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_ !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["nl"], tests::BYTE_SEQ, b"\\^@\\^A\\^B\\^C\\^D\\^E\\^F\\^G\\^H\t\\^J\\^K\\^L\\^M\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_ !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["octal"], tests::BYTE_SEQ, b"\\000\\001\\002\\003\\004\\005\\006\\007\\010\t\n\\013\\014\\015\\016\\017\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037 !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\177\\200\\201\\202\\203\\204\\205\\206\\207\\210\\211\\212\\213\\214\\215\\216\\217\\220\\221\\222\\223\\224\\225\\226\\227\\230\\231\\232\\233\\234\\235\\236\\237\\240\\241\\242\\243\\244\\245\\246\\247\\250\\251\\252\\253\\254\\255\\256\\257\\260\\261\\262\\263\\264\\265\\266\\267\\270\\271\\272\\273\\274\\275\\276\\277\\300\\301\\302\\303\\304\\305\\306\\307\\310\\311\\312\\313\\314\\315\\316\\317\\320\\321\\322\\323\\324\\325\\326\\327\\330\\331\\332\\333\\334\\335\\336\\337\\340\\341\\342\\343\\344\\345\\346\\347\\350\\351\\352\\353\\354\\355\\356\\357\\360\\361\\362\\363\\364\\365\\366\\367\\370\\371\\372\\373\\374\\375\\376\\377");
        check(&["white"], tests::BYTE_SEQ, b"\\^@\\^A\\^B\\^C\\^D\\^E\\^F\\^G\\^H\\^I\\^J\\^K\\^L\\^M\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_\\040!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["glob"], tests::BYTE_SEQ, b"\\^@\\^A\\^B\\^C\\^D\\^E\\^F\\^G\\^H\t\n\\^K\\^L\\^M\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_ !\"\\043$%&'()\\052+,-./0123456789:;<=>\\077@ABCDEFGHIJKLMNOPQRSTUVWXYZ\\133\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["octal"], tests::BYTE_SEQ, b"\\000\\001\\002\\003\\004\\005\\006\\007\\010\t\n\\013\\014\\015\\016\\017\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037 !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\177\\200\\201\\202\\203\\204\\205\\206\\207\\210\\211\\212\\213\\214\\215\\216\\217\\220\\221\\222\\223\\224\\225\\226\\227\\230\\231\\232\\233\\234\\235\\236\\237\\240\\241\\242\\243\\244\\245\\246\\247\\250\\251\\252\\253\\254\\255\\256\\257\\260\\261\\262\\263\\264\\265\\266\\267\\270\\271\\272\\273\\274\\275\\276\\277\\300\\301\\302\\303\\304\\305\\306\\307\\310\\311\\312\\313\\314\\315\\316\\317\\320\\321\\322\\323\\324\\325\\326\\327\\330\\331\\332\\333\\334\\335\\336\\337\\340\\341\\342\\343\\344\\345\\346\\347\\350\\351\\352\\353\\354\\355\\356\\357\\360\\361\\362\\363\\364\\365\\366\\367\\370\\371\\372\\373\\374\\375\\376\\377");
        check(&["octal","white","glob"], tests::BYTE_SEQ, b"\\000\\001\\002\\003\\004\\005\\006\\007\\010\\011\\012\\013\\014\\015\\016\\017\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037\\040!\"\\043$%&'()\\052+,-./0123456789:;<=>\\077@ABCDEFGHIJKLMNOPQRSTUVWXYZ\\133\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\177\\200\\201\\202\\203\\204\\205\\206\\207\\210\\211\\212\\213\\214\\215\\216\\217\\220\\221\\222\\223\\224\\225\\226\\227\\230\\231\\232\\233\\234\\235\\236\\237\\240\\241\\242\\243\\244\\245\\246\\247\\250\\251\\252\\253\\254\\255\\256\\257\\260\\261\\262\\263\\264\\265\\266\\267\\270\\271\\272\\273\\274\\275\\276\\277\\300\\301\\302\\303\\304\\305\\306\\307\\310\\311\\312\\313\\314\\315\\316\\317\\320\\321\\322\\323\\324\\325\\326\\327\\330\\331\\332\\333\\334\\335\\336\\337\\340\\341\\342\\343\\344\\345\\346\\347\\350\\351\\352\\353\\354\\355\\356\\357\\360\\361\\362\\363\\364\\365\\366\\367\\370\\371\\372\\373\\374\\375\\376\\377");
        check(&["cstyle"], tests::BYTE_SEQ, b"\\0\\^A\\^B\\^C\\^D\\^E\\^F\\a\\b\t\n\\v\\f\\r\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_ !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["cstyle","white","glob"], tests::BYTE_SEQ, b"\\0\\^A\\^B\\^C\\^D\\^E\\^F\\a\\b\\t\\n\\v\\f\\r\\^N\\^O\\^P\\^Q\\^R\\^S\\^T\\^U\\^V\\^W\\^X\\^Y\\^Z\\^[\\^\\\\^]\\^^\\^_\\s!\"\\043$%&'()\\052+,-./0123456789:;<=>\\077@ABCDEFGHIJKLMNOPQRSTUVWXYZ\\133\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\^?\\M^@\\M^A\\M^B\\M^C\\M^D\\M^E\\M^F\\M^G\\M^H\\M^I\\M^J\\M^K\\M^L\\M^M\\M^N\\M^O\\M^P\\M^Q\\M^R\\M^S\\M^T\\M^U\\M^V\\M^W\\M^X\\M^Y\\M^Z\\M^[\\M^\\\\M^]\\M^^\\M^_\\240\\M-!\\M-\"\\M-#\\M-$\\M-%\\M-&\\M-'\\M-(\\M-)\\M-*\\M-+\\M-,\\M--\\M-.\\M-/\\M-0\\M-1\\M-2\\M-3\\M-4\\M-5\\M-6\\M-7\\M-8\\M-9\\M-:\\M-;\\M-<\\M-=\\M->\\M-?\\M-@\\M-A\\M-B\\M-C\\M-D\\M-E\\M-F\\M-G\\M-H\\M-I\\M-J\\M-K\\M-L\\M-M\\M-N\\M-O\\M-P\\M-Q\\M-R\\M-S\\M-T\\M-U\\M-V\\M-W\\M-X\\M-Y\\M-Z\\M-[\\M-\\\\M-]\\M-^\\M-_\\M-`\\M-a\\M-b\\M-c\\M-d\\M-e\\M-f\\M-g\\M-h\\M-i\\M-j\\M-k\\M-l\\M-m\\M-n\\M-o\\M-p\\M-q\\M-r\\M-s\\M-t\\M-u\\M-v\\M-w\\M-x\\M-y\\M-z\\M-{\\M-|\\M-}\\M-~\\M^?");
        check(&["octal","cstyle"], tests::BYTE_SEQ, b"\\0\\001\\002\\003\\004\\005\\006\\a\\b\t\n\\v\\f\\r\\016\\017\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037 !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\177\\200\\201\\202\\203\\204\\205\\206\\207\\210\\211\\212\\213\\214\\215\\216\\217\\220\\221\\222\\223\\224\\225\\226\\227\\230\\231\\232\\233\\234\\235\\236\\237\\240\\241\\242\\243\\244\\245\\246\\247\\250\\251\\252\\253\\254\\255\\256\\257\\260\\261\\262\\263\\264\\265\\266\\267\\270\\271\\272\\273\\274\\275\\276\\277\\300\\301\\302\\303\\304\\305\\306\\307\\310\\311\\312\\313\\314\\315\\316\\317\\320\\321\\322\\323\\324\\325\\326\\327\\330\\331\\332\\333\\334\\335\\336\\337\\340\\341\\342\\343\\344\\345\\346\\347\\350\\351\\352\\353\\354\\355\\356\\357\\360\\361\\362\\363\\364\\365\\366\\367\\370\\371\\372\\373\\374\\375\\376\\377");
        check(&["octal","cstyle","white"], tests::BYTE_SEQ, b"\\0\\001\\002\\003\\004\\005\\006\\a\\b\\t\\n\\v\\f\\r\\016\\017\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037\\s!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\177\\200\\201\\202\\203\\204\\205\\206\\207\\210\\211\\212\\213\\214\\215\\216\\217\\220\\221\\222\\223\\224\\225\\226\\227\\230\\231\\232\\233\\234\\235\\236\\237\\240\\241\\242\\243\\244\\245\\246\\247\\250\\251\\252\\253\\254\\255\\256\\257\\260\\261\\262\\263\\264\\265\\266\\267\\270\\271\\272\\273\\274\\275\\276\\277\\300\\301\\302\\303\\304\\305\\306\\307\\310\\311\\312\\313\\314\\315\\316\\317\\320\\321\\322\\323\\324\\325\\326\\327\\330\\331\\332\\333\\334\\335\\336\\337\\340\\341\\342\\343\\344\\345\\346\\347\\350\\351\\352\\353\\354\\355\\356\\357\\360\\361\\362\\363\\364\\365\\366\\367\\370\\371\\372\\373\\374\\375\\376\\377");
        check(&["octal","cstyle","glob"], tests::BYTE_SEQ, b"\\0\\001\\002\\003\\004\\005\\006\\a\\b\t\n\\v\\f\\r\\016\\017\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037 !\"\\043$%&'()\\052+,-./0123456789:;<=>\\077@ABCDEFGHIJKLMNOPQRSTUVWXYZ\\133\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\177\\200\\201\\202\\203\\204\\205\\206\\207\\210\\211\\212\\213\\214\\215\\216\\217\\220\\221\\222\\223\\224\\225\\226\\227\\230\\231\\232\\233\\234\\235\\236\\237\\240\\241\\242\\243\\244\\245\\246\\247\\250\\251\\252\\253\\254\\255\\256\\257\\260\\261\\262\\263\\264\\265\\266\\267\\270\\271\\272\\273\\274\\275\\276\\277\\300\\301\\302\\303\\304\\305\\306\\307\\310\\311\\312\\313\\314\\315\\316\\317\\320\\321\\322\\323\\324\\325\\326\\327\\330\\331\\332\\333\\334\\335\\336\\337\\340\\341\\342\\343\\344\\345\\346\\347\\350\\351\\352\\353\\354\\355\\356\\357\\360\\361\\362\\363\\364\\365\\366\\367\\370\\371\\372\\373\\374\\375\\376\\377");
        check(&["octal","cstyle","white","glob"], tests::BYTE_SEQ, b"\\0\\001\\002\\003\\004\\005\\006\\a\\b\\t\\n\\v\\f\\r\\016\\017\\020\\021\\022\\023\\024\\025\\026\\027\\030\\031\\032\\033\\034\\035\\036\\037\\s!\"\\043$%&'()\\052+,-./0123456789:;<=>\\077@ABCDEFGHIJKLMNOPQRSTUVWXYZ\\133\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\\177\\200\\201\\202\\203\\204\\205\\206\\207\\210\\211\\212\\213\\214\\215\\216\\217\\220\\221\\222\\223\\224\\225\\226\\227\\230\\231\\232\\233\\234\\235\\236\\237\\240\\241\\242\\243\\244\\245\\246\\247\\250\\251\\252\\253\\254\\255\\256\\257\\260\\261\\262\\263\\264\\265\\266\\267\\270\\271\\272\\273\\274\\275\\276\\277\\300\\301\\302\\303\\304\\305\\306\\307\\310\\311\\312\\313\\314\\315\\316\\317\\320\\321\\322\\323\\324\\325\\326\\327\\330\\331\\332\\333\\334\\335\\336\\337\\340\\341\\342\\343\\344\\345\\346\\347\\350\\351\\352\\353\\354\\355\\356\\357\\360\\361\\362\\363\\364\\365\\366\\367\\370\\371\\372\\373\\374\\375\\376\\377");
    }
}
