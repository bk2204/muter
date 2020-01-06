#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::helpers::codecs::StatelessEncoder;
use codec::CodecSettings;
use codec::CodecTransform;
use codec::Error;
use codec::TransformableCodec;
use std::cmp;
use std::collections::BTreeMap;
use std::io;

#[derive(Default)]
pub struct TransformFactory {}

fn transform(inp: &[u8], outp: &mut [u8]) -> (usize, usize) {
    let n = cmp::min(inp.len(), outp.len());
    outp[..n].clone_from_slice(&inp[..n]);
    (n, n)
}

impl TransformFactory {
    pub fn new() -> Self {
        TransformFactory {}
    }
}

impl CodecTransform for TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        Ok(
            StatelessEncoder::new(move |inp, out| transform(inp, out), 1)
                .into_bufread(r, s.bufsize),
        )
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "identity"
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(inp: &[u8]) {
        let reg = CodecRegistry::new();
        for i in vec![4, 5, 6, 7, 8, 512] {
            let c = Chain::new(&reg, "identity", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "identity", i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-identity", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), inp);
            let c = Chain::new(&reg, "-identity", i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), inp);
        }
    }

    #[test]
    fn makes_no_change() {
        check(b"abc");
        check(b"\x00\xff");
        check(b"\xc2\xa9");
        check(b"\x01\x23\x45\x67\x89\xab\xcd\xef");
        check(b"\xfe\xdc\xba");
        check(&(0u8..255u8).collect::<Vec<_>>());
    }

    #[test]
    fn default_tests() {
        tests::round_trip("identity");
        tests::basic_configuration("identity");
        tests::invalid_data("identity");
    }
}
