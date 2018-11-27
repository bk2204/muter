use std::io;
use std::collections::HashMap;
use codec;
use codec::Error;
use codec::CodecSettings;

type TransformFactoryFn = fn(Box<io::BufRead>, CodecSettings) -> Result<Box<io::BufRead>, Error>;

pub struct CodecRegistry {
    map: HashMap<&'static str, TransformFactoryFn>,
}

impl CodecRegistry {
    pub fn new() -> Self {
        let mut map: HashMap<&'static str, TransformFactoryFn> = HashMap::new();

        map.insert("base16", codec::codecs::base16::TransformFactory::factory);
        map.insert("hex", codec::codecs::hex::TransformFactory::factory);
        map.insert("identity",
                   codec::codecs::identity::TransformFactory::factory);
        map.insert("uri", codec::codecs::uri::TransformFactory::factory);

        CodecRegistry { map: map }
    }

    pub fn create<'a>(&self,
                      name: &'a str,
                      r: Box<io::BufRead>,
                      s: CodecSettings)
                      -> Result<Box<io::BufRead>, Error> {
        match self.map.get(name) {
            Some(f) => f(r, s),
            None => Err(Error::UnknownCodec(String::from(name))),
        }
    }
}
