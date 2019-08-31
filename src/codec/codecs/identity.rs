#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::CodecSettings;
use codec::CodecTransform;
use codec::Error;
use codec::StatelessEncoder;
use codec::Transform;
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
    fn factory(&self, r: Box<io::BufRead>, _s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        let enc = StatelessEncoder::new(move |inp, out| transform(inp, out));
        Ok(Box::new(Transform::new(r, enc)))
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        BTreeMap::new()
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "base32"
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;

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
}
