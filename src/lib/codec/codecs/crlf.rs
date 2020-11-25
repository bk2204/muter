#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::helpers::codecs::StatelessEncoder;
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
            Direction::Forward => Ok(StatelessEncoder::new(
                |inp, out| Self::forward_transform(inp, out),
                3,
            )
            .into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(Decoder::new().into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "crlf"
    }
}

impl TransformFactory {
    fn forward_transform(inp: &[u8], outp: &mut [u8]) -> (usize, usize) {
        let n = cmp::min(inp.len(), outp.len() / 2);
        let mut last = 0;
        for (i, x) in inp
            .iter()
            .flat_map(|&x| {
                if x == b'\n' {
                    b"\r\n".to_vec()
                } else {
                    vec![x]
                }
            })
            .enumerate()
        {
            outp[i] = x;
            last = i + 1;
        }
        (n, last)
    }
}

#[derive(Default)]
pub struct Decoder {}

impl Decoder {
    fn new() -> Self {
        Decoder {}
    }
}

impl Codec for Decoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let max = cmp::min(src.len(), dst.len() - 2);
        let mut srcpos = 0;
        let mut dstpos = 0;
        let mut iter = src.iter().take(max).peekable();

        while let Some(incr) = iter.position(|&x| x == b'\r') {
            let src = &src[srcpos..];
            let dst = &mut dst[dstpos..];
            dst[0..incr].copy_from_slice(&src[0..incr]);
            srcpos += incr;
            dstpos += incr;

            let dst = &mut dst[incr..];
            let (srcoff, dstoff) = match (iter.peek(), f) {
                (Some(&&b'\n'), _) => {
                    iter.next();
                    dst[0] = b'\n';
                    (2, 1)
                }
                (Some(&&b'\r'), _) => {
                    dst[0] = b'\r';
                    (1, 1)
                }
                (Some(&&x), _) => {
                    iter.next();
                    dst[0] = b'\r';
                    dst[1] = x;
                    (2, 2)
                }
                (None, FlushState::None) => {
                    return Ok(Status::SeqError(srcpos, dstpos));
                }
                (None, FlushState::Finish) => {
                    dst[dstpos] = b'\r';
                    return Ok(Status::StreamEnd(srcpos + 1, dstpos + 1));
                }
            };
            dstpos += dstoff;
            srcpos += srcoff;
        }

        let pos = max;
        let incr = pos - srcpos;
        let src = &src[srcpos..];
        let dst = &mut dst[dstpos..];
        dst[0..incr].copy_from_slice(&src[0..incr]);
        srcpos += incr;
        dstpos += incr;
        Ok(Status::Ok(srcpos, dstpos))
    }

    fn chunk_size(&self) -> usize {
        2
    }

    fn buffer_size(&self) -> usize {
        3
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![3, 4, 5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, "crlf", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, "-crlf", i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-crlf", i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes() {
        check(b"abc", b"abc");
        check(b"\r", b"\r");
        check(b"\n", b"\r\n");
        check(b"\r\n", b"\r\r\n");
        check(b"\rdef", b"\rdef");
        check(b"\ndef", b"\r\ndef");
        check(b"\r\ndef", b"\r\r\ndef");
        check(b"a\r", b"a\r");
        check(b"a\n", b"a\r\n");
        check(b"a\r\n", b"a\r\r\n");
        check(b"a\rdef", b"a\rdef");
        check(b"a\ndef", b"a\r\ndef");
        check(b"a\r\ndef", b"a\r\r\ndef");
        check(b"ab\r", b"ab\r");
        check(b"ab\n", b"ab\r\n");
        check(b"ab\r\n", b"ab\r\r\n");
        check(b"ab\rdef", b"ab\rdef");
        check(b"ab\ndef", b"ab\r\ndef");
        check(b"ab\r\ndef", b"ab\r\r\ndef");
        check(b"abc\r", b"abc\r");
        check(b"abc\n", b"abc\r\n");
        check(b"abc\r\n", b"abc\r\r\n");
        check(b"abc\rdef", b"abc\rdef");
        check(b"abc\ndef", b"abc\r\ndef");
        check(b"abc\r\ndef", b"abc\r\r\ndef");
        check(b"abcd\r", b"abcd\r");
        check(b"abcd\n", b"abcd\r\n");
        check(b"abcd\r\n", b"abcd\r\r\n");
        check(b"abcd\rdef", b"abcd\rdef");
        check(b"abcd\ndef", b"abcd\r\ndef");
        check(b"abcd\r\ndef", b"abcd\r\r\ndef");
    }

    #[test]
    fn default_tests() {
        tests::round_trip("crlf");
        tests::basic_configuration("crlf");
        tests::invalid_data("crlf");
    }

    #[test]
    fn known_values() {
        check(tests::BYTE_SEQ, b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0d\x0a\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2a\x2b\x2c\x2d\x2e\x2f\x30\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\x3e\x3f\x40\x41\x42\x43\x44\x45\x46\x47\x48\x49\x4a\x4b\x4c\x4d\x4e\x4f\x50\x51\x52\x53\x54\x55\x56\x57\x58\x59\x5a\x5b\x5c\x5d\x5e\x5f\x60\x61\x62\x63\x64\x65\x66\x67\x68\x69\x6a\x6b\x6c\x6d\x6e\x6f\x70\x71\x72\x73\x74\x75\x76\x77\x78\x79\x7a\x7b\x7c\x7d\x7e\x7f\x80\x81\x82\x83\x84\x85\x86\x87\x88\x89\x8a\x8b\x8c\x8d\x8e\x8f\x90\x91\x92\x93\x94\x95\x96\x97\x98\x99\x9a\x9b\x9c\x9d\x9e\x9f\xa0\xa1\xa2\xa3\xa4\xa5\xa6\xa7\xa8\xa9\xaa\xab\xac\xad\xae\xaf\xb0\xb1\xb2\xb3\xb4\xb5\xb6\xb7\xb8\xb9\xba\xbb\xbc\xbd\xbe\xbf\xc0\xc1\xc2\xc3\xc4\xc5\xc6\xc7\xc8\xc9\xca\xcb\xcc\xcd\xce\xcf\xd0\xd1\xd2\xd3\xd4\xd5\xd6\xd7\xd8\xd9\xda\xdb\xdc\xdd\xde\xdf\xe0\xe1\xe2\xe3\xe4\xe5\xe6\xe7\xe8\xe9\xea\xeb\xec\xed\xee\xef\xf0\xf1\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9\xfa\xfb\xfc\xfd\xfe\xff");
    }
}
