#![allow(unknown_lints)]
#![allow(bare_trait_objects)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::manual_range_contains))]

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
pub struct UuencodeTransformFactory {}

impl UuencodeTransformFactory {
    pub fn new() -> Self {
        UuencodeTransformFactory {}
    }
}

impl CodecTransform for UuencodeTransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        match s.dir {
            Direction::Forward => Ok(Encoder::new().into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(Decoder::new(s.strict).into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "uuencode"
    }
}

pub struct Encoder {
    finished: bool,
}

impl Encoder {
    fn new() -> Self {
        Encoder { finished: false }
    }

    fn enc(b: u8) -> u8 {
        if b == 0 {
            b'`'
        } else {
            b + 32
        }
    }

    fn transform_chunk(&self, inp: &[u8], outp: &mut [u8]) -> (usize, usize) {
        let (is, os) = (3, 4);
        let bits = is * 8 / os;
        let mask = (1u64 << bits) - 1;
        let n = cmp::min((inp.len() + is - 1) / is, outp.len() / os);

        outp[0] = Self::enc(inp.len() as u8);

        let (a, b) = (0..n).map(|x| (x * is, x * os)).fold((0, 0), |_, (i, j)| {
            let max = cmp::min(i + is, inp.len());
            let x: u64 = inp[i..max]
                .iter()
                .enumerate()
                .map(|(k, &v)| u64::from(v) << ((is - 1 - k) * 8))
                .sum();

            for (k, val) in outp[j + 1..j + os + 1].iter_mut().enumerate().take(os) {
                *val = Self::enc((x >> ((os - 1 - k) * bits) & mask) as u8);
            }
            (max, j + os)
        });

        outp[b + 1] = b'\n';

        (a, b + 2)
    }
}

impl Codec for Encoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let (is, os) = (45, 62);
        let chunks = cmp::min(inp.len() / is, outp.len() / os);

        if chunks == 0 {
            return match (f, self.finished, inp.len(), outp.len()) {
                (FlushState::Finish, false, 0, len) if len >= 2 => {
                    outp[0..2].copy_from_slice(b"`\n");
                    self.finished = true;
                    Ok(Status::StreamEnd(0, 2))
                }
                (FlushState::Finish, false, _, len) if len >= (os + 2) => {
                    let (a, b) = self.transform_chunk(inp, outp);
                    outp[b..b + 2].copy_from_slice(b"`\n");
                    self.finished = true;
                    Ok(Status::StreamEnd(a, b + 2))
                }
                (FlushState::Finish, true, _, _) => Ok(Status::StreamEnd(0, 0)),
                (_, _, _, _) => Ok(Status::SeqError(0, 0)),
            };
        }
        let ret = (0..chunks).fold(Ok((0, 0)), |_, i| {
            let max = cmp::min((i + 1) * is, inp.len());
            let r = self.transform_chunk(&inp[i * is..max], &mut outp[i * os..(i + 1) * os]);
            Ok((i * is + r.0, i * os + r.1))
        })?;
        Ok(Status::Ok(ret.0, ret.1))
    }

    fn chunk_size(&self) -> usize {
        45
    }

    fn buffer_size(&self) -> usize {
        62
    }
}

#[derive(Default)]
pub struct Decoder {
    strict: bool,
}

impl Decoder {
    fn new(strict: bool) -> Self {
        Self { strict }
    }

    fn dec(b: u8) -> u8 {
        (b.wrapping_sub(32)) & 63
    }

    fn valid_char(b: u8) -> bool {
        b >= 33 && b <= 96
    }

    fn transform_chunk(&self, inp: &[u8], outp: &mut [u8]) -> Result<(usize, usize), Error> {
        let (is, os) = (4, 3);
        let bits = os * 8 / is;

        // If we have a short line, it's possible that our buffer may contain the final "`\n"
        // which we are not equipped to handle, so process just this line, and let the next
        // iteration handle the next line.
        let first = Self::dec(inp[0]) as usize;
        let expected = ((first + (os - 1)) / os) * is + 2;
        if inp.len() < expected || inp[expected - 1] != b'\n' {
            return Err(Error::InvalidSequence("uuencode".to_string(), inp.to_vec()));
        }

        let inp = &inp[0..expected];

        if self.strict {
            let m = inp[0..inp.len() - 1]
                .iter()
                .find(|&&v| !Self::valid_char(v));
            match (m, inp[inp.len() - 1]) {
                (Some(&x), _) => {
                    return Err(Error::InvalidSequence("uuencode".to_string(), vec![x]))
                }
                (None, b'\n') => (),
                (None, c) => return Err(Error::InvalidSequence("uuencode".to_string(), vec![c])),
            };
        };

        let chunks = (inp.len() - 2) / is;

        for (i, j) in (0..chunks).map(|x| (x * is + 1, x * os)) {
            let max = cmp::min(i + is, inp.len() - 1);
            let x: u64 = inp[i..max]
                .iter()
                .enumerate()
                .map(|(k, &v)| u64::from(Self::dec(v)) << ((is - 1 - k) * bits))
                .sum();

            let mut k = 0;
            while k < os && j + k < first {
                outp[j + k] = ((x as u64) >> ((os - 1 - k) * 8) & 0xff) as u8;
                k += 1;
            }
        }
        Ok((inp.len(), first))
    }
}

impl Codec for Decoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let (is, os) = (62, 45);
        let chunks = match f {
            FlushState::None => cmp::min(inp.len() / is, outp.len() / os),
            FlushState::Finish => cmp::min((inp.len() + is - 1) / is, outp.len() / os),
        };

        if chunks == 0 && f == FlushState::None {
            return Ok(Status::SeqError(0, 0));
        }

        let ret = (0..chunks).fold(Ok((0, 0)), |_, i| {
            let max = cmp::min((i + 1) * is, inp.len());
            let r = self.transform_chunk(&inp[i * is..max], &mut outp[i * os..(i + 1) * os])?;
            Ok((i * is + r.0, i * os + r.1))
        })?;
        Ok(Status::Ok(ret.0, ret.1))
    }

    fn chunk_size(&self) -> usize {
        62
    }

    fn buffer_size(&self) -> usize {
        45
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![45, 46, 47, 48, 512] {
            let c = Chain::new(&reg, "uuencode", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
        }

        for i in vec![62, 63, 64, 65, 512] {
            let c = Chain::new(&reg, "-uuencode", i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-uuencode", i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes() {
        check(b"", b"`\n");
        check(b"abc", b"#86)C\n`\n");
        check(
            b"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqr",
            b"L86)C9&5F9VAI:FML;6YO<'%R<W1U=G=X>7IA8F-D969G:&EJ:VQM;F]P<7(`\n`\n",
        );
        check(
            b"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrs",
            b"M86)C9&5F9VAI:FML;6YO<'%R<W1U=G=X>7IA8F-D969G:&EJ:VQM;F]P<7)S\n`\n",
        );
        check(
            b"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrst",
            b"M86)C9&5F9VAI:FML;6YO<'%R<W1U=G=X>7IA8F-D969G:&EJ:VQM;F]P<7)S\n!=```\n`\n",
        );
    }

    #[test]
    fn default_tests() {
        tests::round_trip("uuencode");
        tests::basic_configuration("uuencode");
        tests::invalid_data("uuencode");
    }

    #[test]
    fn known_values() {
        check(tests::BYTE_SEQ, b"M``$\"`P0%!@<(\"0H+#`T.#Q`1$A,4%187&!D:&QP='A\\@(2(C)\"4F)R@I*BLL\nM+2XO,#$R,S0U-C<X.3H[/#T^/T!!0D-$149'2$E*2TQ-3D]045)35%565UA9\nM6EM<75Y?8&%B8V1E9F=H:6IK;&UN;W!Q<G-T=79W>'EZ>WQ]?G^`@8*#A(6&\nMAXB)BHN,C8Z/D)&2DY25EI>8F9J;G)V>GZ\"AHJ.DI::GJ*FJJZRMKJ^PL;*S\nMM+6VM[BYNKN\\O;Z_P,'\"P\\3%QL?(R<K+S,W.S]#1TM/4U=;7V-G:V]S=WM_@\n?X>+CY.7FY^CIZNOL[>[O\\/'R\\_3U]O?X^?K[_/W^_P``\n`\n");
    }
}
