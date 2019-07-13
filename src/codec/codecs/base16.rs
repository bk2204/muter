use codec::CodecSettings;
use codec::Error;
use std::io;
use std::vec;

pub struct TransformFactory {}

impl TransformFactory {
    pub fn factory(r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        let settings = CodecSettings {
            bufsize: s.bufsize,
            strict: s.strict,
            args: vec!["upper"].iter().map(|&x| String::from(x)).collect(),
            dir: s.dir,
        };
        ::codec::codecs::hex::TransformFactory::factory(r, settings)
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;

    fn reg() -> CodecRegistry {
        CodecRegistry::new()
    }

    fn check(inp: &[u8], upper: &[u8]) {
        let c = Chain::new(reg(), "base16", 512, true);
        assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
        let c = Chain::new(reg(), "-base16", 512, true);
        assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
        let c = Chain::new(reg(), "-base16", 512, false);
        assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
    }

    #[test]
    fn encodes_bytes() {
        check(b"abc", b"616263");
        check(b"\x00\xff", b"00FF");
        check(b"\xc2\xa9", b"C2A9");
        check(b"\x01\x23\x45\x67\x89\xab\xcd\xef", b"0123456789ABCDEF");
        check(b"\xfe\xdc\xba", b"FEDCBA");
    }
}
