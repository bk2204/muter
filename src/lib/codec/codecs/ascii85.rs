#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::helpers::codecs::AffixEncoder;
use codec::helpers::codecs::PaddedEncoder;
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

const DIVISORS: [u64; 5] = [85 * 85 * 85 * 85, 85 * 85 * 85, 85 * 85, 85, 1];

#[derive(Default)]
pub struct Ascii85Encoder {}

impl Ascii85Encoder {
    fn new() -> Self {
        Ascii85Encoder {}
    }

    fn pad_bytes_needed(b: usize, is: usize, os: usize) -> usize {
        let r = is - b;
        eprintln!("b={} is={} os={} r={}", b, is, os, r);
        r
    }
}

impl Codec for Ascii85Encoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let (is, os) = (4, 5);
        let mut j = 0;
        let n = cmp::min(inp.len() / is, outp.len() / os);
        for i in (0..n).map(|x| x * is) {
            let x: u64 = inp[i..i + is]
                .iter()
                .enumerate()
                .map(|(k, &v)| u64::from(v) << ((is - 1 - k) * 8))
                .sum();

            if x == 0 && f == FlushState::None {
                outp[j] = b'z';
                j += 1;
            } else {
                for (k, val) in outp[j..j + os].iter_mut().enumerate().take(os) {
                    *val = ((x / DIVISORS[k]) % 85) as u8 + b'!';
                }
                j += 5;
            }
        }
        Ok(Status::Ok(n * is, j))
    }

    fn chunk_size(&self) -> usize {
        4
    }
}

#[derive(Default)]
pub struct Ascii85TransformFactory {}

impl Ascii85TransformFactory {
    pub fn new() -> Self {
        Ascii85TransformFactory {}
    }
}

impl CodecTransform for Ascii85TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => Ok(AffixEncoder::new(
                PaddedEncoder::new_with_pad_function(
                    Ascii85Encoder::new(),
                    4,
                    5,
                    None,
                    Ascii85Encoder::pad_bytes_needed,
                ),
                vec![b'<', b'~'],
                vec![b'~', b'>'],
            )
            .into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(Ascii85Decoder::new().into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "ascii85"
    }
}

pub struct Ascii85Decoder {
    start: bool,
    end: bool,
}

impl Ascii85Decoder {
    fn new() -> Self {
        Ascii85Decoder {
            start: false,
            end: false,
        }
    }

    fn dec(b: u8, i: usize) -> u64 {
        (b.wrapping_sub(b'!') as u64) * DIVISORS[i]
    }

    fn transform_chunk(&self, inp: &[u8], outp: &mut [u8]) -> (usize, usize) {
        let (is, os) = (5, 4);
        let mut tmpbuf = [b'u'; 5];

        if inp[0] == b'z' {
            outp[0..4].copy_from_slice(b"\x00\x00\x00\x00");
            return (1, 4);
        };

        let buf = if inp.len() < is {
            tmpbuf[0..inp.len()].copy_from_slice(inp);
            &tmpbuf
        } else {
            inp
        };

        // We remove as many characters from the output as from the input.
        let off = outp.len() - (is - inp.len());
        eprintln!(
            "inp={:?} inp={} outp={} buf={:?} off={}",
            inp,
            inp.len(),
            outp.len(),
            buf,
            off
        );

        let x: u64 = buf.iter().enumerate().map(|(k, &v)| Self::dec(v, k)).sum();

        for (k, val) in outp[0..off].iter_mut().enumerate() {
            *val = (x >> ((os - 1 - k) * 8) & 0xff) as u8;
        }
        (inp.len(), off)
    }
}

impl Codec for Ascii85Decoder {
    fn transform(
        &mut self,
        src: &[u8],
        dst: &mut [u8],
        flush: FlushState,
    ) -> Result<Status, Error> {
        let (is, os) = (5, 4);
        let prefixlen = 2;

        if !self.start {
            if src.len() < prefixlen {
                return Ok(Status::BufError(0, 0));
            } else {
                self.start = true;
                if &src[0..prefixlen] != b"<~" {
                    return Err(Error::InvalidSequence(
                        "ascii85".to_string(),
                        src[0..prefixlen].to_vec(),
                    ));
                }
                let r = self.transform(&src[prefixlen..], dst, flush)?;
                let (a, b) = r.unpack();
                return Ok(Status::Ok(a + prefixlen, b));
            }
        }

        let loc = src.iter().position(|&x| x == b'~');
        eprintln!("src={:?} loc={:?} end={}", src, loc, self.end);
        let end = if let Some(x) = loc {
            match (x, src.len(), self.end) {
                (_, _, true) => return Err(Error::ExtraData),
                (0, 2, false) => {
                    self.end = true;
                    return Ok(Status::StreamEnd(2, 0));
                }
                (_, len, false) if x + 1 == len => x,
                (_, len, false) if x + 2 == len && src[x + 1] == b'>' => {
                    let r = self.transform(&src[0..x], dst, FlushState::Finish)?;
                    let dstconsumed = r.unpack().1;
                    self.end = true;
                    return Ok(Status::StreamEnd(src.len(), dstconsumed));
                }
                (_, _, false) => {
                    return Err(Error::InvalidSequence(
                        "ascii85".to_string(),
                        src[x..].to_vec(),
                    ))
                }
            }
        } else {
            src.len()
        };

        let maxi = match (flush, end) {
            (FlushState::Finish, x) => x,
            (FlushState::None, x) if x <= is => return Ok(Status::BufError(0, 0)),
            (FlushState::None, x) => x - is,
        };
        let maxj = dst.len() - os;

        let (mut i, mut j) = (0, 0);
        while i < maxi && j < maxj {
            let srcend = cmp::min(i + is, end);
            let (ri, rj) = self.transform_chunk(&src[i..srcend], &mut dst[j..j + os]);
            i += ri;
            j += rj;
        }
        Ok(Status::Ok(i, j))
    }

    fn chunk_size(&self) -> usize {
        5
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(name: &str, inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        let rev = format!("-{}", name);
        for i in vec![5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, name, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, &rev, i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, &rev, i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes_ascii85() {
        check("ascii85", b"", b"<~~>");
        check("ascii85", b"a", b"<~@/~>");
        check("ascii85", b"ab", b"<~@:B~>");
        check("ascii85", b"abc", b"<~@:E^~>");
        check("ascii85", b"abcd", b"<~@:E_W~>");
        check("ascii85", b"\x00", b"<~!!~>");
        check("ascii85", b"\x00\x00", b"<~!!!~>");
        check("ascii85", b"\x00\x00\x00", b"<~!!!!~>");
        check("ascii85", b"\x00\x00\x00\x00", b"<~z~>");
        check(
            "ascii85",
            b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            b"<~zzz~>",
        );
        check("ascii85", b"\xff", b"<~rr~>");
        check("ascii85", b"\xff\xff", b"<~s8N~>");
        check("ascii85", b"\xff\xff\xff", b"<~s8W*~>");
        check("ascii85", b"\xff\xff\xff\xff", b"<~s8W-!~>");
        check(
            "ascii85",
            b"\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff",
            b"<~s8W-!s8W-!s8W-!~>",
        );
        check("ascii85", b"    ", b"<~+<VdL~>");

        // Example from Wikipedia.
        check("ascii85", b"Man is distinguished, not only by his reason, but by this singular passion from other animals, which is a lust of the mind, that by a perseverance of delight in the continued and indefatigable generation of knowledge, exceeds the short vehemence of any carnal pleasure.", br#"<~9jqo^BlbD-BleB1DJ+*+F(f,q/0JhKF<GL>Cj@.4Gp$d7F!,L7@<6@)/0JDEF<G%<+EV:2F!,O<DJ+*.@<*K0@<6L(Df-\0Ec5e;DffZ(EZee.Bl.9pF"AGXBPCsi+DGm>@3BB/F*&OCAfu2/AKYi(DIb:@FD,*)+C]U=@3BN#EcYf8ATD3s@q?d$AftVqCh[NqF<G:8+EV:.+Cf>-FD5W8ARlolDIal(DId<j@<?3r@:F%a+D58'ATD4$Bl@l3De:,-DJs`8ARoFb/0JMK@qB4^F!,R<AKZ&-DfTqBG%G>uD.RTpAKYo'+CT/5+Cei#DII?(E,9)oF*2M7/c~>"#);
    }

    #[test]
    fn default_tests_ascii85() {
        tests::round_trip("ascii85");
        tests::basic_configuration("ascii85");
        tests::invalid_data("ascii85");
    }

    #[test]
    fn known_values_ascii85() {
        check("ascii85", tests::BYTE_SEQ, br#"<~!!*-'"9eu7#RLhG$k3[W&.oNg'GVB"(`=52*$$(B+<_pR,UFcb-n-Vr/1iJ-0JP==1c70M3&s#]4?Ykm5X@_(6q'R884cEH9MJ8X:f1+h<)lt#=BSg3>[:ZC?t!MSA7]@cBPD3sCi+'.E,fo>FEMbNG^4U^I!pHnJ:W<)KS>/9Ll%"IN/`jYOHG]iPa.Q$R$jD4S=Q7DTV8*TUnsrdW2ZetXKAY/Yd(L?['d?O\@K2_]Y2%o^qmn*`5Ta:aN;TJbg"GZd*^:jeCE.%f\,!5gtgiEi8N\UjQ5OekiqBum-X60nF?)@o_%qPq"ad`r;HWp~>"#);
    }
}
