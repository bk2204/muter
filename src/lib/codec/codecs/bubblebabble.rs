#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::helpers::codecs::FilteredDecoder;
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

pub const V: [u8; 6] = *b"aeiouy";
pub const C: [u8; 17] = *b"bcdfghklmnprstvzx";

pub const REVV: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, 0, -1, -1, -1, 1, -1, -1, -1, 2, -1, -1, -1, -1, -1, 3, -1, -1, -1, -1, -1, 4, -1, -1, -1,
    5, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
];
pub const REVC: [i8; 256] = [
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, 0, 1, 2, -1, 3, 4, 5, -1, -1, 6, 7, 8, 9, -1, 10, -1, 11, 12, 13, -1, 14, -1, 16, -1,
    15, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
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
        match s.dir {
            Direction::Forward => Ok(Encoder::new().into_bufread(r, s.bufsize)),
            Direction::Reverse => Ok(Decoder::new(s.strict).into_bufread(r, s.bufsize)),
        }
    }

    fn options(&self) -> BTreeMap<String, String> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "bubblebabble"
    }
}

pub struct Encoder {
    started: bool,
    finished: bool,
    c: usize,
}

impl Encoder {
    fn new() -> Self {
        Encoder {
            started: false,
            finished: false,
            c: 1,
        }
    }

    fn transform_chunk(inp: &[u8], outp: &mut [u8], c: usize) -> usize {
        let inp = &inp[0..2];
        let outp = &mut outp[0..6];

        let c = c % 36;

        outp[0] = V[(((inp[0] as usize >> 6) & 0x3) + c) % 6];
        outp[1] = C[(inp[0] as usize >> 2) & 0xf];
        outp[2] = V[((inp[0] as usize & 0x3) + (c / 6)) % 6];
        outp[3] = C[(inp[1] as usize >> 4) & 0xf];
        outp[4] = b'-';
        outp[5] = C[(inp[1] as usize) & 0xf];

        (c * 5) + (inp[0] as usize * 7) + (inp[1] as usize)
    }

    fn transform_final_chunk(inp: &[u8], outp: &mut [u8], c: usize) {
        let outp = &mut outp[0..4];

        let c = c % 36;

        if inp.is_empty() {
            outp[0] = V[c % 6];
            outp[1] = C[16];
            outp[2] = V[c / 6];
        } else {
            let d = inp[0];
            outp[0] = V[(((d as usize >> 6) & 0x3) + c) % 6];
            outp[1] = C[(d as usize >> 2) & 0xf];
            outp[2] = V[((d as usize & 0x3) + (c / 6)) % 6];
        }
        outp[3] = b'x';
    }
}

impl Codec for Encoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        let (inp, outp, extra) = if !self.started {
            if outp.is_empty() {
                return Ok(Status::SeqError(0, 0));
            }
            self.started = true;
            outp[0] = b'x';
            (inp, &mut outp[1..], 1)
        } else {
            (inp, outp, 0)
        };

        let (is, os) = (2, 6);
        let chunks = cmp::min(inp.len() / is, outp.len() / os);

        if chunks == 0 {
            return match (f, self.finished, inp.len(), outp.len()) {
                (FlushState::Finish, false, _, len) if len >= 4 => {
                    Self::transform_final_chunk(inp, outp, self.c);
                    self.finished = true;
                    Ok(Status::StreamEnd(0, 4 + extra))
                }
                (FlushState::Finish, true, _, _) => Ok(Status::StreamEnd(0, extra)),
                (_, _, _, _) => Ok(Status::SeqError(0, extra)),
            };
        }
        let (il, ol) = (inp.len(), outp.len());
        self.c = (0..chunks).fold(self.c, |c, i| {
            Self::transform_chunk(&inp[i * is..il], &mut outp[i * os..ol], c)
        });
        Ok(Status::Ok(chunks * is, chunks * os + extra))
    }

    fn chunk_size(&self) -> usize {
        2
    }

    fn buffer_size(&self) -> usize {
        7
    }
}

#[derive(Default)]
pub struct Decoder {
    c: usize,
    started: bool,
    finished: bool,
    strict: bool,
}

impl Decoder {
    fn new(strict: bool) -> Self {
        Self {
            c: 1,
            started: false,
            finished: false,
            strict,
        }
    }

    fn transform_chunk(inp: &[u8], outp: &mut [u8], c: usize) -> Result<usize, Error> {
        let inp = &inp[0..6];
        let outp = &mut outp[0..2];

        let c = c % 36;
        let d = [
            REVV[inp[0] as usize],
            REVC[inp[1] as usize],
            REVV[inp[2] as usize],
            REVC[inp[3] as usize],
            REVC[inp[5] as usize],
        ];
        if d.iter().any(|x| *x < 0) || inp[4] != b'-' {
            return Err(Error::InvalidSequence(
                "bubblebabble".to_string(),
                inp.to_vec(),
            ));
        }
        let ax = ((d[0] as u8) + 36 - (c as u8)) % 6;
        let bx = d[1] as u8;
        let cx = ((d[2] as u8) + 6 - ((c as u8) / 6)) % 6;
        if ax >= 4 || cx >= 4 || bx >= 16 {
            return Err(Error::InvalidSequence(
                "bubblebabble".to_string(),
                inp.to_vec(),
            ));
        }
        outp[0] = (ax << 6) | (bx << 2) | cx;
        outp[1] = ((d[3] as u8) << 4) | (d[4] as u8);
        Ok((c * 5) + (outp[0] as usize * 7) + (outp[1] as usize))
    }

    fn transform_final_chunk(inp: &[u8], outp: &mut [u8], c: usize) -> Result<usize, Error> {
        let inp = &inp[0..4];
        let outp = &mut outp[0..1];

        let c = c % 36;
        let d = [
            REVV[inp[0] as usize],
            REVC[inp[1] as usize],
            REVV[inp[2] as usize],
        ];
        if d.iter().any(|x| *x < 0) || inp[3] != b'x' {
            return Err(Error::InvalidSequence(
                "bubblebabble".to_string(),
                inp.to_vec(),
            ));
        }

        if d[1] == 16 {
            if ((d[0] as u8) != ((c as u8) % 6)) || ((d[2] as u8) != ((c as u8) / 6)) {
                return Err(Error::InvalidSequence(
                    "bubblebabble".to_string(),
                    inp.to_vec(),
                ));
            }
            return Ok(0);
        }

        let ax = ((d[0] as u8) + 36 - (c as u8)) % 6;
        let bx = d[1] as u8;
        let cx = ((d[2] as u8) + 6 - ((c as u8) / 6)) % 6;
        if ax >= 4 || cx >= 4 || bx >= 16 {
            return Err(Error::InvalidSequence(
                "bubblebabble".to_string(),
                inp.to_vec(),
            ));
        }
        outp[0] = (ax << 6) | (bx << 2) | cx;
        Ok(1)
    }
}

impl FilteredDecoder for Decoder {
    fn strict(&self) -> bool {
        self.strict
    }

    fn filter_byte(&self, b: u8) -> bool {
        REVV[b as usize] != -1 || REVC[b as usize] != -1 || b == b'-'
    }

    fn internal_transform(
        &mut self,
        inp: &[u8],
        outp: &mut [u8],
        f: FlushState,
    ) -> Result<Status, Error> {
        let (inp, outp, extra) = if !self.started {
            if inp.is_empty() {
                return Ok(Status::SeqError(0, 0));
            }
            self.started = true;
            if inp[0] != b'x' {
                return Err(Error::InvalidSequence(
                    "bubblebabble".to_string(),
                    inp.to_vec(),
                ));
            }
            (&inp[1..], outp, 1)
        } else {
            (inp, outp, 0)
        };

        let (is, os) = (6, 2);
        let chunks = cmp::min(inp.len() / is, outp.len() / os);

        if chunks == 0 {
            return match (f, self.finished, inp.len(), outp.len()) {
                (FlushState::Finish, false, 4, len) if len >= 2 => {
                    let count = Self::transform_final_chunk(inp, outp, self.c)?;
                    self.finished = true;
                    Ok(Status::StreamEnd(4 + extra, count))
                }
                (FlushState::Finish, true, _, _) => Ok(Status::StreamEnd(extra, 0)),
                (_, _, _, _) => Ok(Status::SeqError(extra, 0)),
            };
        }

        let (il, ol) = (inp.len(), outp.len());
        self.c = (0..chunks).fold(Ok(self.c), |res, i| {
            let c = res?;
            Self::transform_chunk(&inp[i * is..il], &mut outp[i * os..ol], c)
        })?;
        Ok(Status::Ok(chunks * is + extra, chunks * os))
    }
}

impl Codec for Decoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        self.wrap_transform(inp, outp, f)
    }

    fn chunk_size(&self) -> usize {
        7
    }

    fn buffer_size(&self) -> usize {
        2
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![2, 3, 4, 5, 6, 7, 512] {
            let c = Chain::new(&reg, "bubblebabble", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
        }

        for i in vec![6, 7, 8, 9, 10, 11, 512] {
            let c = Chain::new(&reg, "-bubblebabble", i, true);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-bubblebabble", i, false);
            assert_eq!(c.transform(outp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn encodes_bytes() {
        check(b"", b"xexax");
        check(b"1234567890", b"xesef-disof-gytuf-katof-movif-baxux");
        check(b"Pineapple", b"xigak-nyryk-humil-bosek-sonax");
        // SHA-1 of "foo".
        check(
            b"\x0b\xee\xc7\xb5\xea\x3f\x0f\xdb\xc9\x5d\x0d\xd4\x7f\x3c\x5b\xc2\x75\xda\x8a\x33",
            b"xedov-vycir-hopof-zofot-radoh-tofyt-gezuf-sikus-dotet-pydif-faxux",
        );
        // MD-5 of "foo".
        check(
            b"\xac\xbd\x18\xdb\x4c\xc2\xf8\x5c\xed\xef\x65\x4f\xcc\xc4\xa4\xd8",
            b"xorar-takyt-rufys-davuh-suruv-zinog-zifos-genet-moxix",
        );
    }

    #[test]
    fn default_tests() {
        tests::round_trip("bubblebabble");
        tests::basic_configuration("bubblebabble");
        tests::invalid_data("bubblebabble");
    }
}
