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
use std::cmp;
use std::collections::BTreeMap;
use std::io;

#[derive(Default)]
pub struct TransformFactory {}

impl TransformFactory {
    pub fn new() -> Self {
        TransformFactory {}
    }
}

impl CodecTransform for TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => (),
            Direction::Reverse => return Err(Error::ForwardOnly("lf".to_string())),
        }
        Ok(Encoder::new(s.args.contains_key("empty")).into_bufread(r, s.bufsize))
    }

    fn options(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert(
            "empty".to_string(),
            tr!("print nothing if the input is empty"),
        );
        map
    }

    fn can_reverse(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "lf"
    }
}

pub struct Encoder {
    last: Option<u8>,
    done: bool,
    empty: bool,
}

impl Codec for Encoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        if self.done {
            return Ok(Status::StreamEnd(0, 0));
        }
        if outp.len() < 2 {
            return Ok(Status::BufError(0, 0));
        }

        // How many bytes should we reserve for the next iteration?
        let to_reserve = match (f, inp.len()) {
            (FlushState::None, 0) => return Ok(Status::SeqError(0, 0)),
            (FlushState::None, _) => 1,
            (FlushState::Finish, 0) => 0,
            (FlushState::Finish, _) => 1,
        };

        // How many bytes are in self.last?
        let lastlen = match self.last {
            Some(_) => 1,
            None => 0,
        };

        // Copy the last item if there was one.
        if let Some(b) = self.last {
            outp[0] = b;
        }

        // Copy the data in common.
        let len = cmp::min(inp.len() - to_reserve, outp.len() - lastlen - 1);
        outp[lastlen..lastlen + len].copy_from_slice(&inp[0..len]);

        let mut nladded = 0;
        if to_reserve == 1 {
            // If we're reserving a byte for the next round, store it.
            self.last = Some(inp[len]);
        } else {
            // We're not reserving a byte for the next round because this is the last time through,
            // so let's see if the last character was a newline.  If not, add one.  The exception
            // is if we have no previous character (and hence an empty input) and the user has
            // requested we not add one in that case.
            self.done = true;
            match (self.last, self.empty) {
                (Some(b'\n'), _) | (None, true) => (),
                (Some(_), _) | (None, false) => {
                    outp[lastlen + len] = b'\n';
                    nladded = 1;
                }
            }
        };
        Ok(Status::Ok(len + to_reserve, len + lastlen + nladded))
    }

    fn chunk_size(&self) -> usize {
        2
    }

    fn buffer_size(&self) -> usize {
        2
    }
}

impl Encoder {
    fn new(empty: bool) -> Self {
        Encoder {
            last: None,
            done: false,
            empty,
        }
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![4, 5, 6, 512] {
            let c = Chain::new(&reg, "lf", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, "lf", i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, "lf(empty)", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, "lf(empty)", i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
        }
    }

    fn check_full(inp: &[u8], def: &[u8], empty: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![4, 5, 6, 512] {
            let c = Chain::new(&reg, "lf", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), def);
            let c = Chain::new(&reg, "lf", i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), def);
            let c = Chain::new(&reg, "lf(empty)", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), empty);
            let c = Chain::new(&reg, "lf(empty)", i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), empty);
        }
    }

    #[test]
    fn expected_values() {
        check_full(b"", b"\n", b"");
        check(b"\n", b"\n");
        check(b"abc", b"abc\n");
        check(b"abc\n", b"abc\n");
        check(b"abcd", b"abcd\n");
        check(b"abcd\n", b"abcd\n");
        check(b"abcde", b"abcde\n");
        check(b"abcde\n", b"abcde\n");
        check(b"abcdef", b"abcdef\n");
        check(b"abcdef\n", b"abcdef\n");
        check(b"abcdefg", b"abcdefg\n");
        check(b"abcdefg\n", b"abcdefg\n");
        check(b"abcdefgh", b"abcdefgh\n");
        check(b"abcdefgh\n", b"abcdefgh\n");
        check(b"abcdefghi", b"abcdefghi\n");
        check(b"abcdefghi\n", b"abcdefghi\n");
        check(b"\xc2\xa9", b"\xc2\xa9\n");
        check(b"\nabcd", b"\nabcd\n");
        check(b"\nabcd\n", b"\nabcd\n");
    }

    #[test]
    fn default_tests() {
        tests::basic_configuration("lf");
    }
}
