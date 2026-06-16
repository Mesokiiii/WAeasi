//! SHAKE128 / SHAKE256 — extendable-output functions (FIPS 202).
//!
//! Identical sponge to SHA-3 but with domain byte `0x1F` and arbitrary
//! output length.  Used as the building block for ML-KEM (Kyber)
//! sampling and ML-DSA (Dilithium) expansion.
use super::keccak::{absorb_block, squeeze, State};

const DOMAIN_SHAKE: u8 = 0x1F;

const fn rate_bytes(security: usize) -> usize { (1600 - 2 * security) / 8 }

const RATE_128: usize = rate_bytes(128);   // 168
const RATE_256: usize = rate_bytes(256);   // 136

pub struct Shake<const RATE: usize> {
    state: State,
    buf:   [u8; 200],
    n:     usize,
    /// Has finalize() already absorbed the padding block?
    squeezing: bool,
}

pub type Shake128 = Shake<RATE_128>;
pub type Shake256 = Shake<RATE_256>;

impl<const RATE: usize> Shake<RATE> {
    pub fn new() -> Self { Self { state: [0; 25], buf: [0; 200], n: 0, squeezing: false } }

    pub fn update(&mut self, mut data: &[u8]) {
        debug_assert!(!self.squeezing, "update() after squeeze()");
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

    /// Squeeze `out.len()` bytes.  Idempotent — multiple calls keep
    /// producing fresh keystream from where the last call ended.
    pub fn squeeze_into(&mut self, out: &mut [u8]) {
        if !self.squeezing {
            for b in &mut self.buf[self.n..RATE] { *b = 0; }
            self.buf[self.n] = DOMAIN_SHAKE;
            self.buf[RATE - 1] |= 0x80;
            absorb_block(&mut self.state, &self.buf, RATE);
            self.squeezing = true;
        }
        squeeze(&mut self.state, RATE, out);
    }

    pub fn finalize_xof(mut self, out_len: usize) -> alloc::vec::Vec<u8> {
        let mut out = alloc::vec![0u8; out_len];
        self.squeeze_into(&mut out);
        out
    }
}

/// One-shot helpers.
pub fn shake128(input: &[u8], out_len: usize) -> alloc::vec::Vec<u8> {
    let mut s = Shake128::new(); s.update(input); s.finalize_xof(out_len)
}
pub fn shake256(input: &[u8], out_len: usize) -> alloc::vec::Vec<u8> {
    let mut s = Shake256::new(); s.update(input); s.finalize_xof(out_len)
}
