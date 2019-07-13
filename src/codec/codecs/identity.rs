use codec::CodecSettings;
use codec::Error;
use codec::StatelessEncoder;
use codec::Transform;
use std::io;

pub struct TransformFactory {}

fn transform(inp: &[u8], outp: &mut [u8]) -> (usize, usize) {
    let n = std::cmp::min(inp.len(), outp.len());
    outp[..n].clone_from_slice(&inp[..n]);
    (n, n)
}

impl TransformFactory {
    pub fn factory(r: Box<io::BufRead>, _s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        let enc = StatelessEncoder::new(move |inp, out| transform(inp, out));
        Ok(Box::new(Transform::new(r, enc)))
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;

    fn reg() -> CodecRegistry {
        CodecRegistry::new()
    }

    fn check(inp: &[u8]) {
        for i in vec![4, 5, 6, 7, 8, 512] {
            let c = Chain::new(reg(), "identity", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), inp);
            let c = Chain::new(reg(), "identity", i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), inp);
            let c = Chain::new(reg(), "-identity", i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), inp);
            let c = Chain::new(reg(), "-identity", i, false);
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
