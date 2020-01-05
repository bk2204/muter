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
        let linelen = s
            .length_arg("length", 1, Some(80))?
            .ok_or_else(|| Error::InvalidArgument("wrap".to_string(), "length".to_string()))?;
        match s.dir {
            Direction::Forward => Ok(Encoder::new(linelen).into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(Decoder::new(s.strict).into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        let mut map = BTreeMap::new();
        map.insert(
            "length".to_string(),
            "wrap at specified line length (default 80)",
        );
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "wrap"
    }
}

#[derive(Default)]
pub struct Encoder {
    curline: usize,
    linelen: usize,
}

impl Encoder {
    pub fn new(linelen: usize) -> Self {
        Encoder {
            curline: 0,
            linelen,
        }
    }

    fn process_chunk(&self, chunkoff: usize, src: &[u8], dst: &mut [u8]) -> (usize, usize, usize) {
        let linelen = self.linelen;
        let mut srcoff = 0;
        let mut dstoff = 0;
        let mut chunkoff = chunkoff;
        while srcoff < src.len() {
            let end = cmp::min(
                self.linelen - chunkoff,
                cmp::min(src.len() - srcoff, linelen),
            );
            let src = &src[srcoff..srcoff + end];
            let dst = &mut dst[dstoff..dstoff + end + 1];

            dst[0..end].copy_from_slice(src);
            if chunkoff + end == linelen {
                dst[end] = b'\n';
                dstoff += 1;
            }
            srcoff += end;
            dstoff += end;
            chunkoff = (chunkoff + end) % self.linelen;
        }
        (srcoff, dstoff, chunkoff)
    }
}

impl Codec for Encoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], _f: FlushState) -> Result<Status, Error> {
        let max = cmp::min(
            src.len(),
            dst.len() - ((dst.len() + self.linelen - 1) / self.linelen) * 2,
        );
        let mut srcpos = 0;
        let mut dstpos = 0;
        let mut iter = src.iter().take(max);

        while let Some(incr) = iter.position(|&x| x == b'\n') {
            let src = &src[srcpos..];
            let dst = &mut dst[dstpos..];
            let (srcoff, mut dstoff, chunkoff) =
                self.process_chunk(self.curline, &src[0..incr], dst);
            if chunkoff % self.linelen != 0 {
                dst[dstoff] = b'\n';
                dstoff += 1;
            }
            srcpos += srcoff + 1;
            dstpos += dstoff;
            self.curline = 0;
        }

        let pos = max;
        let incr = pos - srcpos;
        let src = &src[srcpos..];
        let dst = &mut dst[dstpos..];
        let (srcoff, dstoff, chunkoff) = self.process_chunk(self.curline, &src[0..incr], dst);
        self.curline = chunkoff % self.linelen;
        srcpos += srcoff;
        dstpos += dstoff;
        Ok(Status::Ok(srcpos, dstpos))
    }

    fn chunk_size(&self) -> usize {
        80
    }
}

pub struct Decoder {}

impl Decoder {
    fn new(_strict: bool) -> Self {
        Decoder {}
    }
}

impl Codec for Decoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], _f: FlushState) -> Result<Status, Error> {
        let max = cmp::min(src.len(), dst.len());
        let mut last = 0;
        for (i, x) in src.iter().take(max).filter(|&&x| x != b'\n').enumerate() {
            dst[i] = *x;
            last = i + 1;
        }
        Ok(Status::Ok(max, last))
    }

    fn chunk_size(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;
    use codec::Error;

    fn check(name: &str, inp: &[u8], outp: &[u8], dec: &[u8]) {
        let reg = CodecRegistry::new();
        let reverse = format!("-{}", name);
        for i in vec![6, 7, 8, 9, 10, 11, 512] {
            let c = Chain::new(&reg, name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, &reverse, i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), dec);
            let c = Chain::new(&reg, &reverse, i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), dec);
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
        check_length!("wrap,length", Error::MissingArgument(_));
        check_length!("wrap,length=0", Error::InvalidArgument(_, _));
        check_length!("wrap,length=lalala", Error::InvalidArgument(_, _));
    }

    #[test]
    fn round_trip() {
        tests::basic_configuration("wrap");
        tests::invalid_data("wrap");
    }

    #[test]
    fn wrapping() {
        let b: Vec<u8> = (b'A'..b'Z' + 1).collect();
        check("wrap", &b, b"ABCDEFGHIJKLMNOPQRSTUVWXYZ", &b);
        check("wrap,length=10", &b, b"ABCDEFGHIJ\nKLMNOPQRST\nUVWXYZ", &b);
        check(
            "wrap,length=10",
            b"ABCDEFGHIJ\nKLMNOPQRST\nUVWXYZ",
            b"ABCDEFGHIJ\nKLMNOPQRST\nUVWXYZ",
            &b,
        );
        check(
            "wrap,length=10",
            b"ABCDEFGHIJKLMNOPQRST\nUVWXYZ",
            b"ABCDEFGHIJ\nKLMNOPQRST\nUVWXYZ",
            &b,
        );
        check(
            "wrap,length=10",
            b"ABCDEFGHI\nJKLMNOPQRS\nTUVWXYZ",
            b"ABCDEFGHI\nJKLMNOPQRS\nTUVWXYZ",
            &b,
        );
    }

    #[test]
    fn known_values() {
        check(
            "wrap",
            tests::BYTE_SEQ,
        b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\n\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2a\x2b\x2c\x2d\x2e\x2f\x30\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\x3e\x3f\x40\x41\x42\x43\x44\x45\x46\x47\x48\x49\x4a\x4b\x4c\x4d\x4e\x4f\x50\x51\x52\x53\x54\x55\x56\x57\x58\x59\x5a\n\x5b\x5c\x5d\x5e\x5f\x60\x61\x62\x63\x64\x65\x66\x67\x68\x69\x6a\x6b\x6c\x6d\x6e\x6f\x70\x71\x72\x73\x74\x75\x76\x77\x78\x79\x7a\x7b\x7c\x7d\x7e\x7f\x80\x81\x82\x83\x84\x85\x86\x87\x88\x89\x8a\x8b\x8c\x8d\x8e\x8f\x90\x91\x92\x93\x94\x95\x96\x97\x98\x99\x9a\x9b\x9c\x9d\x9e\x9f\xa0\xa1\xa2\xa3\xa4\xa5\xa6\xa7\xa8\xa9\xaa\n\xab\xac\xad\xae\xaf\xb0\xb1\xb2\xb3\xb4\xb5\xb6\xb7\xb8\xb9\xba\xbb\xbc\xbd\xbe\xbf\xc0\xc1\xc2\xc3\xc4\xc5\xc6\xc7\xc8\xc9\xca\xcb\xcc\xcd\xce\xcf\xd0\xd1\xd2\xd3\xd4\xd5\xd6\xd7\xd8\xd9\xda\xdb\xdc\xdd\xde\xdf\xe0\xe1\xe2\xe3\xe4\xe5\xe6\xe7\xe8\xe9\xea\xeb\xec\xed\xee\xef\xf0\xf1\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9\xfa\n\xfb\xfc\xfd\xfe\xff",
        b"\x00\x01\x02\x03\x04\x05\x06\x07\x08\x09\x0b\x0c\x0d\x0e\x0f\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f\x20\x21\x22\x23\x24\x25\x26\x27\x28\x29\x2a\x2b\x2c\x2d\x2e\x2f\x30\x31\x32\x33\x34\x35\x36\x37\x38\x39\x3a\x3b\x3c\x3d\x3e\x3f\x40\x41\x42\x43\x44\x45\x46\x47\x48\x49\x4a\x4b\x4c\x4d\x4e\x4f\x50\x51\x52\x53\x54\x55\x56\x57\x58\x59\x5a\x5b\x5c\x5d\x5e\x5f\x60\x61\x62\x63\x64\x65\x66\x67\x68\x69\x6a\x6b\x6c\x6d\x6e\x6f\x70\x71\x72\x73\x74\x75\x76\x77\x78\x79\x7a\x7b\x7c\x7d\x7e\x7f\x80\x81\x82\x83\x84\x85\x86\x87\x88\x89\x8a\x8b\x8c\x8d\x8e\x8f\x90\x91\x92\x93\x94\x95\x96\x97\x98\x99\x9a\x9b\x9c\x9d\x9e\x9f\xa0\xa1\xa2\xa3\xa4\xa5\xa6\xa7\xa8\xa9\xaa\xab\xac\xad\xae\xaf\xb0\xb1\xb2\xb3\xb4\xb5\xb6\xb7\xb8\xb9\xba\xbb\xbc\xbd\xbe\xbf\xc0\xc1\xc2\xc3\xc4\xc5\xc6\xc7\xc8\xc9\xca\xcb\xcc\xcd\xce\xcf\xd0\xd1\xd2\xd3\xd4\xd5\xd6\xd7\xd8\xd9\xda\xdb\xdc\xdd\xde\xdf\xe0\xe1\xe2\xe3\xe4\xe5\xe6\xe7\xe8\xe9\xea\xeb\xec\xed\xee\xef\xf0\xf1\xf2\xf3\xf4\xf5\xf6\xf7\xf8\xf9\xfa\xfb\xfc\xfd\xfe\xff",
        );
    }
}
