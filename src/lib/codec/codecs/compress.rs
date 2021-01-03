#![allow(unknown_lints)]
#![allow(bare_trait_objects)]
#![allow(ellipsis_inclusive_range_patterns)]

use codec::CodecSettings;
use codec::CodecTransform;
use codec::Direction;
use codec::Error;
use flate2::bufread::{
    DeflateDecoder, DeflateEncoder, GzDecoder, GzEncoder, ZlibDecoder, ZlibEncoder,
};
use flate2::Compression;
use std::collections::BTreeMap;
use std::io;
use std::io::BufReader;

enum CompressionType {
    Deflate,
    Gzip,
    Zlib,
}

fn generic_factory(
    r: Box<io::BufRead>,
    s: CodecSettings,
    ctype: CompressionType,
) -> Box<io::BufRead> {
    let compr = Compression::new(6);
    match (s.dir, ctype) {
        (Direction::Forward, CompressionType::Deflate) => {
            Box::new(BufReader::new(DeflateEncoder::new(r, compr)))
        }
        (Direction::Forward, CompressionType::Gzip) => {
            Box::new(BufReader::new(GzEncoder::new(r, compr)))
        }
        (Direction::Forward, CompressionType::Zlib) => {
            Box::new(BufReader::new(ZlibEncoder::new(r, compr)))
        }
        (Direction::Reverse, CompressionType::Deflate) => {
            Box::new(BufReader::new(DeflateDecoder::new(r)))
        }
        (Direction::Reverse, CompressionType::Gzip) => Box::new(BufReader::new(GzDecoder::new(r))),
        (Direction::Reverse, CompressionType::Zlib) => {
            Box::new(BufReader::new(ZlibDecoder::new(r)))
        }
    }
}

macro_rules! compress_defn {
    ($ty: ident, $name: expr, $algo: expr) => {
        #[derive(Default)]
        pub struct $ty {}

        impl $ty {
            pub fn new() -> Self {
                $ty {}
            }
        }

        impl CodecTransform for $ty {
            fn factory(
                &self,
                r: Box<io::BufRead>,
                s: CodecSettings,
            ) -> Result<Box<io::BufRead>, Error> {
                Ok(generic_factory(r, s, $algo))
            }

            fn options(&self) -> BTreeMap<String, String> {
                BTreeMap::new()
            }

            fn can_reverse(&self) -> bool {
                true
            }

            fn name(&self) -> &'static str {
                $name
            }
        }
    };
}

compress_defn!(DeflateTransformFactory, "deflate", CompressionType::Deflate);
compress_defn!(ZlibTransformFactory, "zlib", CompressionType::Zlib);
compress_defn!(GzipTransformFactory, "gzip", CompressionType::Gzip);

#[cfg(test)]
mod tests {
    use codec::tests;
    use std::convert::TryInto;

    fn matches_zlib_pattern(encoded: &[u8]) -> bool {
        if encoded.len() < 2 {
            return false;
        }
        // Window size of 32KiB (0x7x); deflate (0xx8)
        if encoded[0] != 0x78 {
            return false;
        }
        // Compression level should be default (our configuration).
        if (encoded[1] & 0xc0) != 0x80 {
            return false;
        }
        // Checksum must be 0 mod 31.
        if (u16::from_be_bytes(encoded[0..2].try_into().unwrap()) % 31) != 0 {
            return false;
        }
        return true;
    }

    fn matches_gzip_pattern(encoded: &[u8]) -> bool {
        if encoded.len() < 8 {
            return false;
        }
        // Bad header.
        if &encoded[0..2] != b"\x1f\x8b" {
            return false;
        }
        // Not using deflate.
        if encoded[2] != 0x08 {
            return false;
        }
        // Reserved bits set.
        if (encoded[3] & 0xe0) != 0 {
            return false;
        }
        // Extra fields set.
        if (encoded[3] & 0xfc) != 0 {
            return false;
        }
        // mtime is nonzero (our configuration).
        if u32::from_be_bytes(encoded[4..8].try_into().unwrap()) != 0 {
            return false;
        }
        return true;
    }

    #[test]
    fn round_trip_deflate() {
        tests::round_trip("deflate");
        tests::basic_configuration("deflate");
        tests::invalid_data("deflate");
    }

    #[test]
    fn round_trip_gzip() {
        tests::round_trip("gzip");
        tests::basic_configuration("gzip");
        tests::invalid_data("gzip");
        tests::matches_pattern("gzip", matches_gzip_pattern);
    }

    #[test]
    fn round_trip_zlib() {
        tests::round_trip("zlib");
        tests::basic_configuration("zlib");
        tests::invalid_data("zlib");
        tests::matches_pattern("zlib", matches_zlib_pattern);
    }
}
