//! ChaCha20 stream — used as a CSPRNG fallback when RDRAND is unavailable.
//!
//! Reference: RFC 8439.  We implement only the key-stream (no AEAD) and
//! seed the state from `rdtsc` + boot entropy.  All operations are in
//! `Wrapping<u32>` semantics — no overflow checks fire.
const SIGMA: [u32; 4] = [0x6170_7865, 0x3320_646E, 0x7962_2D32, 0x6B20_6574];

#[derive(Clone)]
pub struct ChaCha20 {
    state:  [u32; 16],
    block:  [u8; 64],
    used:   usize,
}

impl ChaCha20 {
    pub fn new(key: &[u8; 32], nonce: &[u8; 12]) -> Self {
        let mut state = [0u32; 16];
        state[..4].copy_from_slice(&SIGMA);
        for i in 0..8 {
            state[4 + i] = u32::from_le_bytes(key[i*4..i*4+4].try_into().unwrap());
        }
        state[12] = 0;
        for i in 0..3 {
            state[13 + i] = u32::from_le_bytes(nonce[i*4..i*4+4].try_into().unwrap());
        }
        Self { state, block: [0; 64], used: 64 }
    }

    /// Fill `out` with key-stream bytes.
    pub fn fill(&mut self, out: &mut [u8]) {
        let mut written = 0;
        while written < out.len() {
            if self.used == 64 { self.next_block(); self.used = 0; }
            let take = core::cmp::min(out.len() - written, 64 - self.used);
            out[written..written + take]
                .copy_from_slice(&self.block[self.used..self.used + take]);
            self.used += take;
            written += take;
        }
    }

    fn next_block(&mut self) {
        let mut x = self.state;
        for _ in 0..10 {
            quarter_round(&mut x, 0, 4,  8, 12);
            quarter_round(&mut x, 1, 5,  9, 13);
            quarter_round(&mut x, 2, 6, 10, 14);
            quarter_round(&mut x, 3, 7, 11, 15);
            quarter_round(&mut x, 0, 5, 10, 15);
            quarter_round(&mut x, 1, 6, 11, 12);
            quarter_round(&mut x, 2, 7,  8, 13);
            quarter_round(&mut x, 3, 4,  9, 14);
        }
        for i in 0..16 { x[i] = x[i].wrapping_add(self.state[i]); }
        for i in 0..16 {
            self.block[i*4..i*4+4].copy_from_slice(&x[i].to_le_bytes());
        }
        self.state[12] = self.state[12].wrapping_add(1);
    }
}

#[inline]
fn quarter_round(x: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    x[a] = x[a].wrapping_add(x[b]); x[d] ^= x[a]; x[d] = x[d].rotate_left(16);
    x[c] = x[c].wrapping_add(x[d]); x[b] ^= x[c]; x[b] = x[b].rotate_left(12);
    x[a] = x[a].wrapping_add(x[b]); x[d] ^= x[a]; x[d] = x[d].rotate_left(8);
    x[c] = x[c].wrapping_add(x[d]); x[b] ^= x[c]; x[b] = x[b].rotate_left(7);
}
