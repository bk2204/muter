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
use std::collections::BTreeMap;
use std::io;

use codec::codecs::hex::{REV, UPPER};

#[derive(Copy, Clone)]
enum Characters {
    Identity,
    Encoded,
}

// Identity: b' ', [33, 60], [62, 162]
// Encoded: everything else
const TABLE: [Characters; 256] = [
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Encoded,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Identity,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
    Characters::Encoded,
];

#[derive(Default)]
pub struct TransformFactory {}

impl TransformFactory {
    pub fn new() -> Self {
        TransformFactory {}
    }
}

impl CodecTransform for TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        let linelen = s.length_arg("length", 4, Some(76))?;
        match s.dir {
            Direction::Forward => Ok(Encoder::new(linelen).into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(Decoder::new(s.strict).into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        let mut map = BTreeMap::new();
        map.insert(
            "length".to_string(),
            "wrap at specified line length (default 76; 0 disables)",
        );
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "quotedprintable"
    }
}

#[derive(Default)]
pub struct Encoder {
    curline: usize,
    linelen: Option<usize>,
}

impl Encoder {
    pub fn new(linelen: Option<usize>) -> Self {
        Encoder {
            curline: 0,
            linelen,
        }
    }
}

impl Codec for Encoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], _f: FlushState) -> Result<Status, Error> {
        // 5 for b"=\n" and one encoded character.
        let max = outp.len() - 5;
        let maxin = inp.len();
        let (mut i, mut j) = (0, 0);
        let mut curline = self.curline;
        while i < maxin && j < max {
            let x = inp[i];
            let typ = TABLE[x as usize];
            let enclen = match typ {
                Characters::Identity => 1,
                Characters::Encoded => 3,
            };
            if let Some(linelen) = self.linelen {
                // +1 for b'='.  Note that we don't count the LF, since the RFC says not to.
                if enclen + curline + 1 > linelen {
                    outp[j..j + 2].copy_from_slice(&[b'=', b'\n']);
                    j += 2;
                    curline = 0;
                }
            }
            match typ {
                Characters::Identity => outp[j] = x,
                _ => {
                    outp[j..j + 3].copy_from_slice(&[
                        b'=',
                        UPPER[(x as usize) >> 4],
                        UPPER[(x as usize) & 15],
                    ]);
                }
            }

            i += 1;
            j += enclen;
            curline += enclen;
        }
        self.curline = curline;
        Ok(Status::Ok(i, j))
    }

    fn chunk_size(&self) -> usize {
        76
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
        let mut iter = src.iter().enumerate();
        let mut j = 0;
        while let Some((i, x)) = iter.next() {
            if j >= dst.len() {
                return Ok(Status::Ok(i, j));
            }

            match *x {
                b'=' => {
                    let b1 = iter.next();
                    // Continuation sequence.
                    if let Some((_, &x)) = b1 {
                        if x == b'\n' {
                            continue;
                        }
                    }
                    let b2 = iter.next();
                    match (b1, b2) {
                        (Some((_, c1)), Some((_, c2))) => {
                            let val: i16 =
                                (i16::from(REV[*c1 as usize]) << 4) | i16::from(REV[*c2 as usize]);
                            if val < 0 {
                                return Err(Error::InvalidSequence(
                                    "quotedprintable".to_string(),
                                    vec![*c1, *c2],
                                ));
                            }
                            dst[j] = val as u8;
                        }
                        _ => return Ok(Status::BufError(i, j)),
                    }
                }
                _ => dst[j] = *x,
            }
            j += 1;
        }
        Ok(Status::Ok(src.len(), j))
    }

    fn chunk_size(&self) -> usize {
        3
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
        let reverse = format!("-{}", name);
        for i in vec![76, 77, 78, 79, 512] {
            let c = Chain::new(&reg, name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, &reverse, i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, &reverse, i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    macro_rules! check_failure {
        ($inp:expr, $x:pat) => {
            let reg = CodecRegistry::new();
            for i in vec![76, 77, 78, 79, 512] {
                for b in vec![true, false] {
                    let c = Chain::new(&reg, "-quotedprintable", i, b);
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

    fn check_std(inp: &[u8], outp: &[u8]) {
        check("quotedprintable", inp, outp);
    }

    #[test]
    fn encodes_bytes() {
        check_std(b"abc", b"abc");
        check_std(b"\x00\xff", b"=00=FF");
        check_std(b"\xc2\xa9", b"=C2=A9");
        check_std(b"\x01\x23\x45\x67\x89\xab\xcd\xef", b"=01#Eg=89=AB=CD=EF");
        check_std(b"\xfe\xdc\xba", b"=FE=DC=BA");
    }

    #[test]
    fn rejects_invalid() {
        check_failure!(b"abc=0xff", Error::InvalidSequence(_, _));
        check_failure!(b"abc=", Error::TruncatedData);
        check_failure!(b"abc=0", Error::TruncatedData);
        check_failure!(b"abc=vv", Error::InvalidSequence(_, _));
    }

    #[test]
    fn rejects_invalid_length() {
        check_length!("quotedprintable,length", Error::MissingArgument(_));
        check_length!("quotedprintable,length=3", Error::InvalidArgument(_, _));
        check_length!(
            "quotedprintable,length=lalala",
            Error::InvalidArgument(_, _)
        );
    }

    #[test]
    fn round_trip() {
        tests::round_trip("quotedprintable");
        tests::basic_configuration("quotedprintable");
        tests::invalid_data("quotedprintable");
    }

    #[test]
    fn wrapping() {
        let b: Vec<u8> = (0..32).collect();
        check("quotedprintable", &b, b"=00=01=02=03=04=05=06=07=08=09=0A=0B=0C=0D=0E=0F=10=11=12=13=14=15=16=17=18=\n=19=1A=1B=1C=1D=1E=1F");
        check("quotedprintable,length=76", &b, b"=00=01=02=03=04=05=06=07=08=09=0A=0B=0C=0D=0E=0F=10=11=12=13=14=15=16=17=18=\n=19=1A=1B=1C=1D=1E=1F");
        check("quotedprintable,length=0", &b, b"=00=01=02=03=04=05=06=07=08=09=0A=0B=0C=0D=0E=0F=10=11=12=13=14=15=16=17=18=19=1A=1B=1C=1D=1E=1F");
        check("quotedprintable,length=7", &b, b"=00=01=\n=02=03=\n=04=05=\n=06=07=\n=08=09=\n=0A=0B=\n=0C=0D=\n=0E=0F=\n=10=11=\n=12=13=\n=14=15=\n=16=17=\n=18=19=\n=1A=1B=\n=1C=1D=\n=1E=1F");
    }

    #[test]
    fn known_values() {
        check_std(
            tests::BYTE_SEQ,
            br##"=00=01=02=03=04=05=06=07=08=09=0A=0B=0C=0D=0E=0F=10=11=12=13=14=15=16=17=18=
=19=1A=1B=1C=1D=1E=1F !"#$%&'()*+,-./0123456789:;<=3D>?@ABCDEFGHIJKLMNOPQRS=
TUVWXYZ[\]^_`abcdefghijklmnopqrstuvwxyz{|}~=7F=80=81=82=83=84=85=86=87=88=
=89=8A=8B=8C=8D=8E=8F=90=91=92=93=94=95=96=97=98=99=9A=9B=9C=9D=9E=9F=A0=A1=
=A2=A3=A4=A5=A6=A7=A8=A9=AA=AB=AC=AD=AE=AF=B0=B1=B2=B3=B4=B5=B6=B7=B8=B9=BA=
=BB=BC=BD=BE=BF=C0=C1=C2=C3=C4=C5=C6=C7=C8=C9=CA=CB=CC=CD=CE=CF=D0=D1=D2=D3=
=D4=D5=D6=D7=D8=D9=DA=DB=DC=DD=DE=DF=E0=E1=E2=E3=E4=E5=E6=E7=E8=E9=EA=EB=EC=
=ED=EE=EF=F0=F1=F2=F3=F4=F5=F6=F7=F8=F9=FA=FB=FC=FD=FE=FF"##,
        );
    }
}
