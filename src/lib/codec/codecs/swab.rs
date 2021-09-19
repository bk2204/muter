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
            Direction::Forward => {
                let chunklen = s
                    .length_arg("length", 1, None)?
                    .ok_or_else(|| Error::MissingArgument("swab".to_string()))?;
                Ok(Encoder::new(chunklen).into_bufread(r, s.bufsize))
            }
            Direction::Reverse => Err(Error::ForwardOnly("swab".to_string())),
        }
    }

    fn options(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("length".to_string(), tr!("handle chunks of this size"));
        map
    }

    fn can_reverse(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "swab"
    }
}

#[derive(Default)]
pub struct Encoder {
    chunklen: usize,
}

impl Encoder {
    pub fn new(chunklen: usize) -> Self {
        Encoder { chunklen }
    }

    fn process_chunk(&self, src: &[u8], dst: &mut [u8]) {
        let src = &src[0..self.chunklen];
        let dst = &mut dst[0..self.chunklen];
        for (d, s) in dst.iter_mut().rev().zip(src.iter()) {
            *d = *s;
        }
    }
}

impl Codec for Encoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let chunk = cmp::min(src.len(), dst.len());
        let iters = chunk / self.chunklen;
        let consumed = iters * self.chunklen;
        match f {
            FlushState::None if src.len() < self.chunklen => {
                return Ok(Status::SeqError(0, 0));
            }
            FlushState::Finish if src.is_empty() => {
                return Ok(Status::StreamEnd(0, 0));
            }
            _ => (),
        }
        for i in 0..iters {
            let (begin, end) = (i * self.chunklen, (i + 1) * self.chunklen);
            self.process_chunk(&src[begin..end], &mut dst[begin..end]);
        }
        match f {
            FlushState::Finish if consumed == src.len() => {
                Ok(Status::StreamEnd(consumed, consumed))
            }
            _ => Ok(Status::Ok(consumed, consumed)),
        }
    }

    fn chunk_size(&self) -> usize {
        self.chunklen
    }

    fn buffer_size(&self) -> usize {
        self.chunklen
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;
    use codec::Error;

    fn check(name: &str, inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![8, 9, 10, 11, 512] {
            let c = Chain::new(&reg, name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
        }
    }

    macro_rules! check_length {
        ($inp:expr, $x:pat) => {
            let reg = CodecRegistry::new();
            let c = Chain::new(&reg, $inp, 512, false);
            match c.transform((0..32).collect()) {
                Ok(_) => panic!("got success for invalid sequence"),
                Err(e) => match e.get_ref().unwrap().downcast_ref::<Error>() {
                    Some(&$x) => (),
                    Some(e) => panic!("got wrong error: {:?}", e),
                    None => panic!("No internal error?"),
                },
            }
        };
    }

    #[test]
    fn rejects_invalid_length() {
        check_length!("swab", Error::MissingArgument(_));
        check_length!("swab,length", Error::MissingArgument(_));
        check_length!("swab,length=lalala", Error::InvalidArgument(_, _));
    }

    #[test]
    fn default_tests() {
        tests::basic_configuration_without_options("swab");
    }

    #[test]
    fn simple_sequences() {
        let b: Vec<u8> = (b'A'..b'X' + 1).collect();
        check("swab,length=2", &b, b"BADCFEHGJILKNMPORQTSVUXW");
        check("swab,length=4", &b, b"DCBAHGFELKJIPONMTSRQXWVU");
        check("swab,length=6", &b, b"FEDCBALKJIHGRQPONMXWVUTS");
        check("swab,length=8", &b, b"HGFEDCBAPONMLKJIXWVUTSRQ");
    }

    #[test]
    fn known_values() {
        check("swab,length=1", tests::BYTE_SEQ, tests::BYTE_SEQ);
        check(
            "swab,length=2",
            tests::BYTE_SEQ,
            b"\x01\x00\x03\x02\x05\x04\x07\x06\x09\x08\x0b\x0a\x0d\x0c\x0f\x0e\x11\x10\x13\x12\x15\x14\x17\x16\x19\x18\x1b\x1a\x1d\x1c\x1f\x1e\x21\x20\x23\x22\x25\x24\x27\x26\x29\x28\x2b\x2a\x2d\x2c\x2f\x2e\x31\x30\x33\x32\x35\x34\x37\x36\x39\x38\x3b\x3a\x3d\x3c\x3f\x3e\x41\x40\x43\x42\x45\x44\x47\x46\x49\x48\x4b\x4a\x4d\x4c\x4f\x4e\x51\x50\x53\x52\x55\x54\x57\x56\x59\x58\x5b\x5a\x5d\x5c\x5f\x5e\x61\x60\x63\x62\x65\x64\x67\x66\x69\x68\x6b\x6a\x6d\x6c\x6f\x6e\x71\x70\x73\x72\x75\x74\x77\x76\x79\x78\x7b\x7a\x7d\x7c\x7f\x7e\x81\x80\x83\x82\x85\x84\x87\x86\x89\x88\x8b\x8a\x8d\x8c\x8f\x8e\x91\x90\x93\x92\x95\x94\x97\x96\x99\x98\x9b\x9a\x9d\x9c\x9f\x9e\xa1\xa0\xa3\xa2\xa5\xa4\xa7\xa6\xa9\xa8\xab\xaa\xad\xac\xaf\xae\xb1\xb0\xb3\xb2\xb5\xb4\xb7\xb6\xb9\xb8\xbb\xba\xbd\xbc\xbf\xbe\xc1\xc0\xc3\xc2\xc5\xc4\xc7\xc6\xc9\xc8\xcb\xca\xcd\xcc\xcf\xce\xd1\xd0\xd3\xd2\xd5\xd4\xd7\xd6\xd9\xd8\xdb\xda\xdd\xdc\xdf\xde\xe1\xe0\xe3\xe2\xe5\xe4\xe7\xe6\xe9\xe8\xeb\xea\xed\xec\xef\xee\xf1\xf0\xf3\xf2\xf5\xf4\xf7\xf6\xf9\xf8\xfb\xfa\xfd\xfc\xff\xfe",
        );
        check(
            "swab,length=4",
            tests::BYTE_SEQ,
            b"\x03\x02\x01\x00\x07\x06\x05\x04\x0b\x0a\x09\x08\x0f\x0e\x0d\x0c\x13\x12\x11\x10\x17\x16\x15\x14\x1b\x1a\x19\x18\x1f\x1e\x1d\x1c\x23\x22\x21\x20\x27\x26\x25\x24\x2b\x2a\x29\x28\x2f\x2e\x2d\x2c\x33\x32\x31\x30\x37\x36\x35\x34\x3b\x3a\x39\x38\x3f\x3e\x3d\x3c\x43\x42\x41\x40\x47\x46\x45\x44\x4b\x4a\x49\x48\x4f\x4e\x4d\x4c\x53\x52\x51\x50\x57\x56\x55\x54\x5b\x5a\x59\x58\x5f\x5e\x5d\x5c\x63\x62\x61\x60\x67\x66\x65\x64\x6b\x6a\x69\x68\x6f\x6e\x6d\x6c\x73\x72\x71\x70\x77\x76\x75\x74\x7b\x7a\x79\x78\x7f\x7e\x7d\x7c\x83\x82\x81\x80\x87\x86\x85\x84\x8b\x8a\x89\x88\x8f\x8e\x8d\x8c\x93\x92\x91\x90\x97\x96\x95\x94\x9b\x9a\x99\x98\x9f\x9e\x9d\x9c\xa3\xa2\xa1\xa0\xa7\xa6\xa5\xa4\xab\xaa\xa9\xa8\xaf\xae\xad\xac\xb3\xb2\xb1\xb0\xb7\xb6\xb5\xb4\xbb\xba\xb9\xb8\xbf\xbe\xbd\xbc\xc3\xc2\xc1\xc0\xc7\xc6\xc5\xc4\xcb\xca\xc9\xc8\xcf\xce\xcd\xcc\xd3\xd2\xd1\xd0\xd7\xd6\xd5\xd4\xdb\xda\xd9\xd8\xdf\xde\xdd\xdc\xe3\xe2\xe1\xe0\xe7\xe6\xe5\xe4\xeb\xea\xe9\xe8\xef\xee\xed\xec\xf3\xf2\xf1\xf0\xf7\xf6\xf5\xf4\xfb\xfa\xf9\xf8\xff\xfe\xfd\xfc",
        );
        check(
            "swab,length=8",
            tests::BYTE_SEQ,
            b"\x07\x06\x05\x04\x03\x02\x01\x00\x0f\x0e\x0d\x0c\x0b\x0a\x09\x08\x17\x16\x15\x14\x13\x12\x11\x10\x1f\x1e\x1d\x1c\x1b\x1a\x19\x18\x27\x26\x25\x24\x23\x22\x21\x20\x2f\x2e\x2d\x2c\x2b\x2a\x29\x28\x37\x36\x35\x34\x33\x32\x31\x30\x3f\x3e\x3d\x3c\x3b\x3a\x39\x38\x47\x46\x45\x44\x43\x42\x41\x40\x4f\x4e\x4d\x4c\x4b\x4a\x49\x48\x57\x56\x55\x54\x53\x52\x51\x50\x5f\x5e\x5d\x5c\x5b\x5a\x59\x58\x67\x66\x65\x64\x63\x62\x61\x60\x6f\x6e\x6d\x6c\x6b\x6a\x69\x68\x77\x76\x75\x74\x73\x72\x71\x70\x7f\x7e\x7d\x7c\x7b\x7a\x79\x78\x87\x86\x85\x84\x83\x82\x81\x80\x8f\x8e\x8d\x8c\x8b\x8a\x89\x88\x97\x96\x95\x94\x93\x92\x91\x90\x9f\x9e\x9d\x9c\x9b\x9a\x99\x98\xa7\xa6\xa5\xa4\xa3\xa2\xa1\xa0\xaf\xae\xad\xac\xab\xaa\xa9\xa8\xb7\xb6\xb5\xb4\xb3\xb2\xb1\xb0\xbf\xbe\xbd\xbc\xbb\xba\xb9\xb8\xc7\xc6\xc5\xc4\xc3\xc2\xc1\xc0\xcf\xce\xcd\xcc\xcb\xca\xc9\xc8\xd7\xd6\xd5\xd4\xd3\xd2\xd1\xd0\xdf\xde\xdd\xdc\xdb\xda\xd9\xd8\xe7\xe6\xe5\xe4\xe3\xe2\xe1\xe0\xef\xee\xed\xec\xeb\xea\xe9\xe8\xf7\xf6\xf5\xf4\xf3\xf2\xf1\xf0\xff\xfe\xfd\xfc\xfb\xfa\xf9\xf8",
        );
    }
}
