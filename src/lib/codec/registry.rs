use codec;
use codec::CodecSettings;
use codec::CodecTransform;
use codec::Error;
use std::collections::btree_map;
use std::collections::BTreeMap;
use std::io;

type Iter<'a> = btree_map::Iter<'a, &'static str, Box<CodecTransform>>;

#[derive(Default)]
pub struct CodecRegistry {
    map: BTreeMap<&'static str, Box<CodecTransform>>,
}

impl CodecRegistry {
    pub fn new() -> Self {
        let mut map: BTreeMap<&'static str, Box<CodecTransform>> = BTreeMap::new();

        map.insert(
            "ascii85",
            Box::new(codec::codecs::ascii85::Ascii85TransformFactory::new()),
        );
        map.insert(
            "base16",
            Box::new(codec::codecs::base16::TransformFactory::new()),
        );
        map.insert(
            "base32",
            Box::new(codec::codecs::base32::Base32TransformFactory::new()),
        );
        map.insert(
            "base32hex",
            Box::new(codec::codecs::base32::Base32HexTransformFactory::new()),
        );
        map.insert(
            "base64",
            Box::new(codec::codecs::base64::Base64TransformFactory::new()),
        );
        map.insert(
            "bubblebabble",
            Box::new(codec::codecs::bubblebabble::TransformFactory::new()),
        );
        map.insert(
            "checksum",
            Box::new(codec::codecs::checksum::TransformFactory::new()),
        );
        map.insert(
            "crlf",
            Box::new(codec::codecs::crlf::TransformFactory::new()),
        );
        map.insert(
            "deflate",
            Box::new(codec::codecs::compress::DeflateTransformFactory::new()),
        );
        map.insert(
            "form",
            Box::new(codec::codecs::uri::FormTransformFactory::new()),
        );
        map.insert(
            "gzip",
            Box::new(codec::codecs::compress::GzipTransformFactory::new()),
        );
        map.insert(
            "hash",
            Box::new(codec::codecs::hash::TransformFactory::new()),
        );
        map.insert("hex", Box::new(codec::codecs::hex::TransformFactory::new()));
        map.insert(
            "identity",
            Box::new(codec::codecs::identity::TransformFactory::new()),
        );
        map.insert("lf", Box::new(codec::codecs::lf::TransformFactory::new()));
        map.insert(
            "modhex",
            Box::new(codec::codecs::hex::ModHexTransformFactory::new()),
        );
        map.insert(
            "quotedprintable",
            Box::new(codec::codecs::quotedprintable::TransformFactory::new()),
        );
        map.insert(
            "uri",
            Box::new(codec::codecs::uri::URITransformFactory::new()),
        );
        map.insert(
            "url64",
            Box::new(codec::codecs::base64::URL64TransformFactory::new()),
        );
        map.insert(
            "uuencode",
            Box::new(codec::codecs::uuencode::UuencodeTransformFactory::new()),
        );
        map.insert(
            "vis",
            Box::new(codec::codecs::vis::VisTransformFactory::new()),
        );
        map.insert(
            "wrap",
            Box::new(codec::codecs::wrap::TransformFactory::new()),
        );
        map.insert(
            "xml",
            Box::new(codec::codecs::xml::XMLTransformFactory::new()),
        );
        map.insert(
            "zlib",
            Box::new(codec::codecs::compress::ZlibTransformFactory::new()),
        );

        CodecRegistry { map }
    }

    pub fn insert(&mut self, k: &'static str, f: Box<CodecTransform>) {
        self.map.insert(k, f);
    }

    pub fn iter<'a>(&'a self) -> Iter<'a> {
        self.map.iter()
    }

    pub fn create<'a>(
        &self,
        name: &'a str,
        r: Box<io::BufRead>,
        s: CodecSettings,
    ) -> Result<Box<io::BufRead>, Error> {
        match self.map.get(name) {
            Some(t) => t.factory(r, s),
            None => Err(Error::UnknownCodec(String::from(name))),
        }
    }
}

#[cfg(test)]
mod tests {
    use codec::registry::CodecRegistry;
    use codec::CodecSettings;
    use codec::CodecTransform;
    use codec::Direction;
    use codec::Error;
    use std::collections::BTreeMap;
    use std::io;
    use std::io::Read;

    struct TestCodecFactory {}

    impl CodecTransform for TestCodecFactory {
        fn factory(
            &self,
            _r: Box<io::BufRead>,
            _s: CodecSettings,
        ) -> Result<Box<io::BufRead>, Error> {
            return Ok(Box::new(io::Cursor::new(vec![0x61, 0x62, 0x63])));
        }

        fn options(&self) -> BTreeMap<String, String> {
            BTreeMap::new()
        }

        fn can_reverse(&self) -> bool {
            true
        }

        fn name(&self) -> &'static str {
            "random"
        }
    }

    fn codec_settings() -> CodecSettings {
        CodecSettings {
            args: BTreeMap::new(),
            bufsize: 512,
            dir: Direction::Forward,
            strict: true,
        }
    }

    #[test]
    fn can_fetch_insertable() {
        let mut cr = CodecRegistry::new();
        let r = Box::new(io::Cursor::new(Vec::new()));

        match cr.create("random", r, codec_settings()) {
            Ok(_) => panic!("unexpected success"),
            Err(Error::UnknownCodec(s)) => assert_eq!(s, "random"),
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        cr.insert("random", Box::new(TestCodecFactory {}));

        let r = Box::new(io::Cursor::new(Vec::new()));
        match cr.create("random", r, codec_settings()) {
            Ok(c) => {
                let c: Result<Vec<u8>, _> = c.bytes().collect();
                let v = c.unwrap();
                assert_eq!(v, vec![0x61, 0x62, 0x63]);
            }
            Err(_) => {
                panic!("failed to insert random");
            }
        }
    }

    #[test]
    fn iterates_in_sorted_order() {
        let cr = CodecRegistry::new();
        let keys: Vec<_> = cr.iter().map(|(key, _)| key).collect();
        let mut keys_sorted = keys.clone();
        keys_sorted.sort();

        assert_eq!(keys, keys_sorted);
    }
}
