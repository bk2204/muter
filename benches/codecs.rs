#![feature(test)]
extern crate muter;
extern crate test;

#[cfg(test)]
mod tests {
    use muter::chain::Chain;
    use muter::codec::registry::CodecRegistry;
    use muter::codec::tests::BYTE_SEQ;
    use test::Bencher;

    const BUFFER_SIZE: usize = 8192;
    const CHUNK_SIZE: usize = BUFFER_SIZE * 2;

    fn large_sequence() -> Vec<u8> {
        // TODO: use repeat when all versions support it.
        let copies = CHUNK_SIZE / BYTE_SEQ.len();
        let mut inp = Vec::with_capacity(BYTE_SEQ.len() * copies);
        for _ in 0..copies {
            inp.extend_from_slice(BYTE_SEQ);
        }
        inp
    }

    fn benchmark_forward(b: &mut Bencher, chain: &str) {
        let reg = CodecRegistry::new();
        let c = Chain::new(&reg, &chain, BUFFER_SIZE, false);
        let inp = large_sequence();
        b.iter(|| c.transform(inp.clone()).unwrap());
    }

    fn benchmark_reverse(b: &mut Bencher, chain: &str, strict: bool) {
        let reg = CodecRegistry::new();
        let c = Chain::new(&reg, &chain, BUFFER_SIZE, strict);
        let seq = large_sequence();
        let inp = c.transform(seq).unwrap();

        let rev = format!("-{}", chain);
        let c = Chain::new(&reg, &rev, BUFFER_SIZE, strict);
        b.iter(|| c.transform(inp.clone()).unwrap());
    }

    macro_rules! benchmark {
        ($chain:expr, $name:ident) => {
            #[cfg(test)]
            mod $name {
                use super::{benchmark_forward, benchmark_reverse};
                use test::Bencher;

                #[bench]
                fn forward(b: &mut Bencher) {
                    benchmark_forward(b, $chain);
                }

                #[bench]
                fn reverse(b: &mut Bencher) {
                    benchmark_reverse(b, $chain, false);
                }

                #[bench]
                fn reverse_strict(b: &mut Bencher) {
                    benchmark_reverse(b, $chain, true);
                }
            }
        };
    }
    benchmark!("ascii85", ascii85);
    benchmark!("base16", base16);
    benchmark!("base32", base32);
    benchmark!("base32hex", base32hex);
    benchmark!("base64", base64);
    benchmark!("form", form);
    benchmark!("hex", hex);
    benchmark!("identity", identity);
    benchmark!("quotedprintable", quotedprintable);
    benchmark!("uri", uri);
    benchmark!("url64", url64);
    benchmark!("uuencode", uuencode);
    benchmark!("vis", vis);
    benchmark!("xml", xml);
}
