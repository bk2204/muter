#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::helpers::codecs::FilteredDecoder;
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

#[derive(Default)]
pub struct ModHexTransformFactory {}

pub const LOWER: [u8; 16] = [
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66,
];
pub const UPPER: [u8; 16] = [
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46,
];
pub const MODHEX: [u8; 16] = *b"cbdefghijklnrtuv";

pub const REV: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, -1, -1, -1, -1, -1, -1, -1, 10, 11, 12, 13, 14, 15, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 10,
    11, 12, 13, 14, 15, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

pub const MODHEXREV: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, 1, 0, 2, 3, 4, 5, 6, 7, 8, 9, 10, -1, 11, -1, -1, -1, 12, -1, 13, 14, 15, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];

fn forward_transform(inp: &[u8], outp: &mut [u8], arr: &[u8; 16]) -> (usize, usize) {
    let n = cmp::min(inp.len(), outp.len() / 2);
    for (i, j) in (0..n).map(|x| (x, x * 2)) {
        outp[j..j + 2]
            .copy_from_slice(&[arr[(inp[i] >> 4) as usize], arr[(inp[i] & 0xf) as usize]]);
    }
    (n, n * 2)
}

impl TransformFactory {
    pub fn new() -> Self {
        TransformFactory {}
    }
}

impl CodecTransform for TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => {
                let arr = if s.bool_arg("upper")? { &UPPER } else { &LOWER };
                Ok(
                    StatelessEncoder::new(move |inp, out| forward_transform(inp, out, arr), 2)
                        .into_bufread(r, s.bufsize),
                )
            }
            Direction::Reverse => Ok(Decoder::new(s.strict, &REV).into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        map.insert("lower".to_string(), tr!("use lowercase letters"));
        map.insert("upper".to_string(), tr!("use uppercase letters"));
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "hex"
    }
}

impl ModHexTransformFactory {
    pub fn new() -> Self {
        ModHexTransformFactory {}
    }
}

impl CodecTransform for ModHexTransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => Ok(StatelessEncoder::new(
                move |inp, out| forward_transform(inp, out, &MODHEX),
                2,
            )
            .into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(Decoder::new(s.strict, &MODHEXREV).into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "modhex"
    }
}

pub struct Decoder {
    strict: bool,
    rev: &'static [i8; 256],
}

impl Decoder {
    fn new(strict: bool, rev: &'static [i8; 256]) -> Self {
        Decoder { strict, rev }
    }
}

impl FilteredDecoder for Decoder {
    fn strict(&self) -> bool {
        self.strict
    }

    fn filter_byte(&self, b: u8) -> bool {
        self.rev[b as usize] != -1
    }

    fn internal_transform(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        f: FlushState,
    ) -> Result<Status, Error> {
        match f {
            FlushState::None if src.len() < 2 => {
                return Ok(Status::SeqError(0, 0));
            }
            FlushState::Finish if src.is_empty() => {
                return Ok(Status::StreamEnd(0, 0));
            }
            _ => (),
        }

        let bytes = cmp::min(src.len() / 2, dst.len());
        let mut consumed = 0;
        for (i, j) in (0..bytes).map(|x| (x * 2, x)) {
            let (x, y) = (src[i], src[i + 1]);
            let v: i16 = (i16::from(self.rev[x as usize]) << 4) | i16::from(self.rev[y as usize]);
            if v < 0 {
                return Err(Error::InvalidSequence("hex".to_string(), vec![x, y]));
            }
            dst[j] = (v & 0xff) as u8;
            consumed = i + 2;
        }

        match f {
            FlushState::Finish if consumed == src.len() => Ok(Status::StreamEnd(consumed, bytes)),
            _ => Ok(Status::Ok(consumed, bytes)),
        }
    }
}

impl Codec for Decoder {
    fn transform(&mut self, src: &[u8], dst: &mut [u8], f: FlushState) -> Result<Status, Error> {
        self.wrap_transform(src, dst, f)
    }

    fn chunk_size(&self) -> usize {
        2
    }

    fn buffer_size(&self) -> usize {
        1
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(inp: &[u8], lower: &[u8], upper: &[u8], modhex: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, "hex", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), lower);
            let c = Chain::new(&reg, "hex,lower", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), lower);
            let c = Chain::new(&reg, "hex,upper", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
            let c = Chain::new(&reg, "-hex", i, true);
            assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-hex", i, true);
            assert_eq!(c.transform(lower.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-hex", i, false);
            assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "modhex", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), modhex);
            let c = Chain::new(&reg, "-modhex", i, true);
            assert_eq!(c.transform(modhex.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-modhex", i, false);
            assert_eq!(c.transform(modhex.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes() {
        check(b"abc", b"616263", b"616263", b"hbhdhe");
        check(b"\x00\xff", b"00ff", b"00FF", b"ccvv");
        check(b"\xc2\xa9", b"c2a9", b"C2A9", b"rdlk");
        check(
            b"\x01\x23\x45\x67\x89\xab\xcd\xef",
            b"0123456789abcdef",
            b"0123456789ABCDEF",
            b"cbdefghijklnrtuv",
        );
        check(b"\xfe\xdc\xba", b"fedcba", b"FEDCBA", b"vutrnl");
    }

    #[test]
    fn default_tests() {
        tests::round_trip("hex");
        tests::round_trip("hex,upper");
        tests::round_trip("hex,lower");
        tests::round_trip_stripped_whitespace("hex");
        tests::round_trip_stripped_whitespace("hex,upper");
        tests::round_trip_stripped_whitespace("hex,lower");
        tests::basic_configuration("hex");
        tests::invalid_data("hex");

        tests::round_trip("modhex");
        tests::basic_configuration("modhex");
        tests::round_trip_stripped_whitespace("modhex");
        tests::invalid_data("modhex");
    }

    #[test]
    fn known_values() {
        check(tests::BYTE_SEQ, b"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f404142434445464748494a4b4c4d4e4f505152535455565758595a5b5c5d5e5f606162636465666768696a6b6c6d6e6f707172737475767778797a7b7c7d7e7f808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9fa0a1a2a3a4a5a6a7a8a9aaabacadaeafb0b1b2b3b4b5b6b7b8b9babbbcbdbebfc0c1c2c3c4c5c6c7c8c9cacbcccdcecfd0d1d2d3d4d5d6d7d8d9dadbdcdddedfe0e1e2e3e4e5e6e7e8e9eaebecedeeeff0f1f2f3f4f5f6f7f8f9fafbfcfdfeff", b"000102030405060708090A0B0C0D0E0F101112131415161718191A1B1C1D1E1F202122232425262728292A2B2C2D2E2F303132333435363738393A3B3C3D3E3F404142434445464748494A4B4C4D4E4F505152535455565758595A5B5C5D5E5F606162636465666768696A6B6C6D6E6F707172737475767778797A7B7C7D7E7F808182838485868788898A8B8C8D8E8F909192939495969798999A9B9C9D9E9FA0A1A2A3A4A5A6A7A8A9AAABACADAEAFB0B1B2B3B4B5B6B7B8B9BABBBCBDBEBFC0C1C2C3C4C5C6C7C8C9CACBCCCDCECFD0D1D2D3D4D5D6D7D8D9DADBDCDDDEDFE0E1E2E3E4E5E6E7E8E9EAEBECEDEEEFF0F1F2F3F4F5F6F7F8F9FAFBFCFDFEFF", b"cccbcdcecfcgchcicjckclcncrctcucvbcbbbdbebfbgbhbibjbkblbnbrbtbubvdcdbdddedfdgdhdidjdkdldndrdtdudvecebedeeefegeheiejekelenereteuevfcfbfdfefffgfhfifjfkflfnfrftfufvgcgbgdgegfggghgigjgkglgngrgtgugvhchbhdhehfhghhhihjhkhlhnhrhthuhvicibidieifigihiiijikiliniritiuivjcjbjdjejfjgjhjijjjkjljnjrjtjujvkckbkdkekfkgkhkikjkkklknkrktkukvlclbldlelflglhliljlklllnlrltlulvncnbndnenfngnhninjnknlnnnrntnunvrcrbrdrerfrgrhrirjrkrlrnrrrtrurvtctbtdtetftgthtitjtktltntrtttutvucubudueufuguhuiujukulunurutuuuvvcvbvdvevfvgvhvivjvkvlvnvrvtvuvv");
    }
}
