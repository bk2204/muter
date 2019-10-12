#![allow(unknown_lints)]
#![allow(bare_trait_objects)]

use blake2::{Blake2b, Blake2s};
use codec::Codec;
use codec::CodecSettings;
use codec::CodecTransform;
use codec::Direction;
use codec::Error;
use codec::FlushState;
use codec::Status;
use codec::TransformableCodec;
use digest::{Digest, DynDigest};
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};
use std::collections::BTreeMap;
use std::io;

#[derive(Default)]
pub struct TransformFactory {}

impl TransformFactory {
    pub fn new() -> Self {
        TransformFactory {}
    }
}

impl TransformFactory {
    fn digest(name: &str) -> Result<Box<DynDigest>, Error> {
        match name {
            "md5" => Ok(Box::new(Md5::new())),
            "sha1" => Ok(Box::new(Sha1::new())),
            "sha224" => Ok(Box::new(Sha224::new())),
            "sha256" => Ok(Box::new(Sha256::new())),
            "sha384" => Ok(Box::new(Sha384::new())),
            "sha512" => Ok(Box::new(Sha512::new())),
            "sha3-224" => Ok(Box::new(Sha3_224::new())),
            "sha3-256" => Ok(Box::new(Sha3_256::new())),
            "sha3-384" => Ok(Box::new(Sha3_384::new())),
            "sha3-512" => Ok(Box::new(Sha3_512::new())),
            "blake2b" => Ok(Box::new(Blake2b::new())),
            "blake2s" => Ok(Box::new(Blake2s::new())),
            _ => Err(Error::UnknownArgument(name.to_string())),
        }
    }
}

impl CodecTransform for TransformFactory {
    fn factory(&self, r: Box<io::BufRead>, s: CodecSettings) -> Result<Box<io::BufRead>, Error> {
        let args: Vec<_> = s.args.iter().collect();
        match args.len() {
            0 => return Err(Error::MissingArgument("hash".to_string())),
            1 => (),
            _ => {
                return Err(Error::IncompatibleParameters(
                    args[0].to_string(),
                    args[1].to_string(),
                ));
            }
        };
        match s.dir {
            Direction::Forward => {
                Ok(Encoder::new(Self::digest(args[0])?).into_bufread(r, s.bufsize))
            }
            Direction::Reverse => Err(Error::ForwardOnly("hash".to_string())),
        }
    }

    fn options(&self) -> BTreeMap<String, &'static str> {
        let mut map = BTreeMap::new();
        map.insert("md5".to_string(), "use MD5 as the hash");
        map.insert("sha1".to_string(), "use SHA-1 as the hash");
        map.insert("sha224".to_string(), "use SHA-224 as the hash");
        map.insert("sha256".to_string(), "use SHA-256 as the hash");
        map.insert("sha384".to_string(), "use SHA-384 as the hash");
        map.insert("sha512".to_string(), "use SHA-512 as the hash");
        map.insert("sha3-224".to_string(), "use SHA3-224 as the hash");
        map.insert("sha3-256".to_string(), "use SHA3-256 as the hash");
        map.insert("sha3-384".to_string(), "use SHA3-384 as the hash");
        map.insert("sha3-512".to_string(), "use SHA3-512 as the hash");
        map.insert("blake2b".to_string(), "use BLAKE2b as the hash");
        map.insert("blake2s".to_string(), "use BLAKE2s as the hash");
        map
    }

    fn can_reverse(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "hex"
    }
}

pub struct Encoder {
    digest: Box<DynDigest>,
    done: bool,
}

impl Codec for Encoder {
    fn transform(&mut self, inp: &[u8], outp: &mut [u8], f: FlushState) -> Result<Status, Error> {
        if self.done {
            return Ok(Status::StreamEnd(0, 0));
        }
        self.digest.input(inp);
        match f {
            FlushState::None => Ok(Status::Ok(inp.len(), 0)),
            FlushState::Finish => {
                let digestlen = self.digest.output_size();
                if outp.len() < digestlen {
                    Ok(Status::BufError(inp.len(), 0))
                } else {
                    outp[0..digestlen].copy_from_slice(&self.digest.result_reset());
                    self.done = true;
                    Ok(Status::StreamEnd(inp.len(), digestlen))
                }
            }
        }
    }

    fn chunk_size(&self) -> usize {
        1
    }
}

impl Encoder {
    pub fn new(digest: Box<DynDigest>) -> Self {
        Encoder {
            digest,
            done: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use chain::Chain;
    use codec::registry::CodecRegistry;

    fn check(algo: &str, inp: &[u8], outp: &[u8]) {
        let reg = CodecRegistry::new();
        let codec = format!("hash({}):hex", algo);
        let dlen = outp.len() / 2;
        for i in vec![dlen, dlen + 1, dlen + 2, dlen + 3, 512] {
            let c = Chain::new(&reg, &codec, i, true);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
            let c = Chain::new(&reg, &codec, i, false);
            assert_eq!(c.transform(inp.to_vec()).unwrap(), outp);
        }
    }

    #[test]
    fn expected_values() {
        let buf = [b'a'; 1000003];
        let items: &[(&str, &[u8], &[u8], &[u8], &[u8])] = &[
            (
                "md5",
                b"d41d8cd98f00b204e9800998ecf8427e",
                b"900150983cd24fb0d6963f7d28e17f72",
                b"f96b697d7cb7938d525a2f31aaf161d0",
                b"5abe51b61ad88ec96601dfaccdc33969"
            ),
            (
                "sha1",
                b"da39a3ee5e6b4b0d3255bfef95601890afd80709",
                b"a9993e364706816aba3e25717850c26c9cd0d89d",
                b"c12252ceda8be8994d5fa0290a47231c1d16aae3",
                b"e0184932e09d5304faec6c3df30a3b8df233ee35"
            ),
            (
                "sha224",
                b"d14a028c2a3a2bc9476102bb288234c415a2b01f828ea62ac5b3e42f",
                b"23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7",
                b"2cb21c83ae2f004de7e81c3c7019cbcb65b71ab656b22d6d0c39b8eb",
                b"dfb7e9002167835eb5278c6842db0bef1c3e6d95b1f0850ba04c75ce"
            ),
            (
                "sha256",
                b"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                b"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
                b"f7846f55cf23e14eebeab5b4e1550cad5b509e3348fbc4efa3a1413d393cb650",
                b"cf5f310cac13bb0f1de288d09940e8ee441e8d5d88b0fd4ec6021c405fda8c8c"
            ),
            (
                "sha384",
                b"38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b",
                b"cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
                b"473ed35167ec1f5d8e550368a3db39be54639f828868e9454c239fc8b52e3c61dbd0d8b4de1390c256dcbb5d5fd99cd5",
                b"fac5a6517d770ef8815a6a732053eb5b0a37216e0af84c23424fa975115002713252b570d191765906ee728f3bdfdb39"
            ),
            (
                "sha512",
                b"cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
                b"ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
                b"107dbf389d9e9f71a3a95f6c055b9251bc5268c2be16d6c13492ea45b0199f3309e16455ab1e96118e8a905d5597b72038ddb372a89826046de66687bb420e7c",
                b"2c85515cd9c21e21ef55d09f4a057fc6881f16827db2e0ea6fe6177495eb6e9b5e466667ffccb7c7653e2b48b38e85589562c2d8f6a26ee5308d05463476adf5"
            ),

            (
                "sha3-224",
                b"6b4e03423667dbb73b6e15454f0eb1abd4597f9a1b078e3f5b5a6bc7",
                b"e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf",
                b"18768bb4c48eb7fc88e5ddb17efcf2964abd7798a39d86a4b4a1e4c8",
                b"4081cc87770ad9eadefd38ea22fb9fc07715937a3ac786588897cc4b"
            ),
            (
                "sha3-256",
                b"a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a",
                b"3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
                b"edcdb2069366e75243860c18c3a11465eca34bce6143d30c8665cefcfd32bffd",
                b"8cea78b15140dd4e1739cf5ab8cb6adf748f8479dcee1a2b0114d8e702a4e5cf"
            ),
            (
                "sha3-384",
                b"0c63a75b845e4f7d01107d852e4c2485c51a50aaaa94fc61995e71bbee983a2ac3713831264adb47fb6bd1e058d5f004",
                b"ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f539f1edf228376d25",
                b"d9519709f44af73e2c8e291109a979de3d61dc02bf69def7fbffdfffe662751513f19ad57e17d4b93ba1e484fc1980d5",
                b"6cf31cc2d9c4249f845664c084e74c596cbf24593a239d584dc011a5aa51be8cc199a53985e55a6a9f039519fd863233"
            ),
            (
                "sha3-512",
                b"a69f73cca23a9ac5c8b567dc185a756e97c982164fe25859e0d1dcc1475c80a615b2123af1f5f94c11e3e9402c3ac558f500199d95b6d3e301758586281dcd26",
                b"b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c91a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
                b"3444e155881fa15511f57726c7d7cfe80302a7433067b29d59a71415ca9dd141ac892d310bc4d78128c98fda839d18d7f0556f2fe7acb3c0cda4bff3a25f5f59",
                b"91df770447926954227ebcfeaa0afbc7ec19a5860e11b25c2505824ab22ee604f56b3239284d1a955a7fcc99c66e469c61d6078fd2ed71d445e2ba06994a81e1"
            ),
            (
                "blake2b",
                b"786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce",
                b"ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923",
                b"3c26ce487b1c0f062363afa3c675ebdbf5f4ef9bdc022cfbef91e3111cdc283840d8331fc30a8a0906cff4bcdbcd230c61aaec60fdfad457ed96b709a382359a",
                b"61c31edfd5786d52aeee64a113edbf3fe1094bf02158eef18d40bbf6aeccd886dd7534e74d80aee1c34c39fef394f47b0cb361892e538cbb05874ab5dd824749"
            ),
            (
                "blake2s",
                b"69217a3079908094e11121d042354a7c1f55b6482ca1a51e1b250dfd1ed0eef9",
                b"508c5e8c327c14e2e1a72ba34eeb452f37458b209ed63a294d999b4c86675982",
                b"fa10ab775acf89b7d3c8a6e823d586f6b67bdbac4ce207fe145b7d3ac25cd28c",
                b"c19280e2aa8a82a3c717b2f9ecbebfb1d559f8896d9916d1b955ce849ff40aa2"
            ),
        ];
        for &(algo, empty, abc, md, lotsa) in items {
            check(algo, b"", empty);
            check(algo, b"abc", abc);
            check(algo, b"message digest", md);
            check(algo, &buf, lotsa);
        }
    }
}
