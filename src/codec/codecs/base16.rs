#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use codec::CodecSettings;
use codec::CodecTransform;
use codec::Error;
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
        let settings = CodecSettings {
            bufsize: s.bufsize,
            strict: s.strict,
            args: vec!["upper"].iter().map(|&x| String::from(x)).collect(),
            dir: s.dir,
        };
        ::codec::codecs::hex::TransformFactory::new().factory(r, settings)
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        let mut map = BTreeMap::new();
        map.insert("lower".to_string(), "use lowercase letters");
        map.insert("upper".to_string(), "use uppercase letters");
        map
    }

    fn can_reverse(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "base16"
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;
    use codec::tests;

    fn check(inp: &[u8], upper: &[u8]) {
        let reg = CodecRegistry::new();
        let c = Chain::new(&reg, "base16", 512, true);
        assert_eq!(c.transform(inp.to_vec()).unwrap(), upper);
        let c = Chain::new(&reg, "-base16", 512, true);
        assert_eq!(c.transform(upper.to_vec()).unwrap(), inp);
        let c = Chain::new(&reg, "-base16", 512, false);
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

    #[test]
    fn default_tests() {
        tests::round_trip("base16");
        tests::basic_configuration("base16");
        tests::invalid_data("base16");
    }
}
