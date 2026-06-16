//! SHA-3 family — SHA3-256, SHA3-384, SHA3-512.
//!
//! Sponge construction over Keccak-f[1600] with capacity = 2 × output
//! bits (FIPS 202 §6).  Output domain-separator byte = `0x06`.
use super::keccak::{absorb_block, squeeze, State};

/// Sponge capacity (in bytes) for output-bit `n`: capacity = 2n / 8.
const fn rate_bytes(out_bits: usize) -> usize { (1600 - 2 * out_bits) / 8 }

const RATE_256: usize = rate_bytes(256);   // 136
const RATE_384: usize = rate_bytes(384);   // 104
const RATE_512: usize = rate_bytes(512);   // 72

const DOMAIN_SHA3: u8 = 0x06;

pub struct Sha3<const RATE: usize, const OUT: usize> {
    state: State,
    buf:   [u8; 200],
    n:     usize,
}

pub type Sha3_256 = Sha3<RATE_256, 32>;
pub type Sha3_384 = Sha3<RATE_384, 48>;
pub type Sha3_512 = Sha3<RATE_512, 64>;

impl<const RATE: usize, const OUT: usize> Sha3<RATE, OUT> {
    pub fn new() -> Self { Self { state: [0; 25], buf: [0; 200], n: 0 } }

    pub fn update(&mut self, mut data: &[u8]) {
        if self.n > 0 {
            let take = (RATE - self.n).min(data.len());
            self.buf[self.n..self.n + take].copy_from_slice(&data[..take]);
            self.n += take;
            data = &data[take..];
            if self.n == RATE {
                absorb_block(&mut self.state, &self.buf, RATE);
                self.n = 0;
            }
        }
        while data.len() >= RATE {
            absorb_block(&mut self.state, &data[..RATE], RATE);
            data = &data[RATE..];
        }
        if !data.is_empty() {
            self.buf[..data.len()].copy_from_slice(data);
            self.n = data.len();
        }
    }

    pub fn finalize(mut self) -> [u8; OUT] {
        // Pad: 0x06 ‖ 0x00 ... 0x80 (XOR'd into last byte of rate).
        for b in &mut self.buf[self.n..RATE] { *b = 0; }
        self.buf[self.n] = DOMAIN_SHA3;
        self.buf[RATE - 1] |= 0x80;
        absorb_block(&mut self.state, &self.buf, RATE);

        let mut out = [0u8; OUT];
        squeeze(&mut self.state, RATE, &mut out);
        out
    }
}

#[inline] pub fn sha3_256(data: &[u8]) -> [u8; 32] {
    let mut h = Sha3_256::new(); h.update(data); h.finalize()
}
#[inline] pub fn sha3_384(data: &[u8]) -> [u8; 48] {
    let mut h = Sha3_384::new(); h.update(data); h.finalize()
}
#[inline] pub fn sha3_512(data: &[u8]) -> [u8; 64] {
    let mut h = Sha3_512::new(); h.update(data); h.finalize()
}

/// `permute` re-export — used by SHAKE in `super::shake`.
pub use super::keccak::permute as keccak_permute;
