use std::io;
use codec;
use codec::Error;
use codec::CodecSettings;

pub struct CodecRegistry {}

impl CodecRegistry {
    pub fn create<'a>(name: &'a str,
                      r: Box<io::BufRead>,
                      s: CodecSettings)
                      -> Result<Box<io::BufRead>, Error> {
        match name {
            "hex" => codec::codecs::hex::TransformFactory::factory(r, s),
            "uri" => codec::codecs::uri::TransformFactory::factory(r, s),
            _ => Err(Error::UnknownCodec(String::from(name))),
        }
    }
}
