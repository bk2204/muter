use codec;
use codec::CodecSettings;
use codec::Error;
use std::collections::HashMap;
use std::io;

type TransformFactoryFn = fn(Box<io::BufRead>, CodecSettings) -> Result<Box<io::BufRead>, Error>;

#[derive(Default)]
pub struct CodecRegistry {
    map: HashMap<&'static str, TransformFactoryFn>,
}

impl CodecRegistry {
    pub fn new() -> Self {
        let mut map: HashMap<&'static str, TransformFactoryFn> = HashMap::new();

        map.insert("base16", codec::codecs::base16::TransformFactory::factory);
        map.insert("base32", codec::codecs::base32::TransformFactory::factory);
        map.insert("base64", codec::codecs::base64::TransformFactory::factory);
        map.insert("hex", codec::codecs::hex::TransformFactory::factory);
        map.insert(
            "identity",
            codec::codecs::identity::TransformFactory::factory,
        );
        map.insert("uri", codec::codecs::uri::TransformFactory::factory);

        CodecRegistry { map }
    }

    pub fn insert(&mut self, k: &'static str, f: TransformFactoryFn) {
        self.map.insert(k, f);
    }

    pub fn create<'a>(
        &self,
        name: &'a str,
        r: Box<io::BufRead>,
        s: CodecSettings,
    ) -> Result<Box<io::BufRead>, Error> {
        match self.map.get(name) {
            Some(f) => f(r, s),
            None => Err(Error::UnknownCodec(String::from(name))),
        }
    }
}

#[cfg(test)]
mod tests {
    use codec::registry::CodecRegistry;
    use codec::CodecSettings;
    use codec::Direction;
    use codec::Error;
    use std::collections::BTreeSet;
    use std::io;
    use std::io::Read;

    fn factory(_r: Box<io::BufRead>, _s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        return Ok(Box::new(io::Cursor::new(vec![0x61, 0x62, 0x63])));
    }

    fn codec_settings() -> CodecSettings {
        CodecSettings {
            args: BTreeSet::new(),
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

        cr.insert("random", factory);

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
}
