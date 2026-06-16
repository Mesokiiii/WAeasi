//! BLAKE3 — fast cryptographic hash (RFC-style spec, 2020).
//!
//! Stage-9 ships the **single-threaded** reference implementation:
//!   * 16-word block (64 bytes), 8-word output (32 bytes).
//!   * Compression function = 7 rounds of `g`.
//!   * Tree-mode chunk chaining is left for stage-10 (when SIMD lands).
//!
//! Why BLAKE3 in 2026: 3-5x faster than SHA-256 on modern CPUs,
//! parallelizable, used by `cargo`, IPFS, Wireguard alternatives.

const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
    0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

const MSG_PERMUTATION: [usize; 16] = [2,6,3,10,7,0,4,13,1,11,12,5,9,14,15,8];

const CHUNK_START:        u32 = 1 << 0;
const CHUNK_END:          u32 = 1 << 1;
const ROOT:               u32 = 1 << 3;
const KEYED_HASH:         u32 = 1 << 4;
const DERIVE_KEY_CONTEXT: u32 = 1 << 5;
const DERIVE_KEY_MATERIAL:u32 = 1 << 6;

const BLOCK_LEN: usize = 64;
const CHUNK_LEN: usize = 1024;
const OUT_LEN:   usize = 32;

#[inline]
fn g(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(mx);
    state[d] = (state[d] ^ state[a]).rotate_right(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(12);
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(my);
    state[d] = (state[d] ^ state[a]).rotate_right(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(7);
}

fn round(state: &mut [u32; 16], m: &[u32; 16]) {
    g(state, 0, 4, 8,12, m[0],  m[1]);
    g(state, 1, 5, 9,13, m[2],  m[3]);
    g(state, 2, 6,10,14, m[4],  m[5]);
    g(state, 3, 7,11,15, m[6],  m[7]);
    g(state, 0, 5,10,15, m[8],  m[9]);
    g(state, 1, 6,11,12, m[10], m[11]);
    g(state, 2, 7, 8,13, m[12], m[13]);
    g(state, 3, 4, 9,14, m[14], m[15]);
}

fn permute(m: &mut [u32; 16]) {
    let mut p = [0u32; 16];
    for i in 0..16 { p[i] = m[MSG_PERMUTATION[i]]; }
    *m = p;
}

fn compress(cv: &[u32; 8], block_words: &[u32; 16], counter: u64,
            block_len: u32, flags: u32) -> [u32; 16] {
    let counter_lo = counter as u32;
    let counter_hi = (counter >> 32) as u32;
    let mut state = [
        cv[0], cv[1], cv[2], cv[3],
        cv[4], cv[5], cv[6], cv[7],
        IV[0], IV[1], IV[2], IV[3],
        counter_lo, counter_hi, block_len, flags,
    ];
    let mut m = *block_words;
    for _ in 0..6 { round(&mut state, &m); permute(&mut m); }
    round(&mut state, &m);
    for i in 0..8 {
        state[i]     ^= state[i + 8];
        state[i + 8] ^= cv[i];
    }
    state
}

pub struct Blake3 {
    cv:        [u32; 8],
    chunk_buf: [u8; BLOCK_LEN],
    chunk_n:   usize,
    block_count_in_chunk: u8,
    bytes_total: u64,
}

impl Blake3 {
    pub fn new() -> Self {
        Self { cv: IV, chunk_buf: [0; BLOCK_LEN], chunk_n: 0,
               block_count_in_chunk: 0, bytes_total: 0 }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            if self.chunk_n == BLOCK_LEN {
                self.compress_block(false);
                self.chunk_n = 0;
            }
            let take = (BLOCK_LEN - self.chunk_n).min(data.len());
            self.chunk_buf[self.chunk_n..self.chunk_n + take]
                .copy_from_slice(&data[..take]);
            self.chunk_n += take;
            self.bytes_total += take as u64;
            data = &data[take..];
        }
    }

    fn compress_block(&mut self, root: bool) {
        let mut block_words = [0u32; 16];
        for i in 0..16 {
            block_words[i] = u32::from_le_bytes(
                self.chunk_buf[i*4..i*4+4].try_into().unwrap());
        }
        let mut flags = 0u32;
        if self.block_count_in_chunk == 0 { flags |= CHUNK_START; }
        if root { flags |= CHUNK_END | ROOT; }
        let out = compress(&self.cv, &block_words, 0,
                           self.chunk_n as u32, flags);
        self.cv = [out[0],out[1],out[2],out[3],out[4],out[5],out[6],out[7]];
        self.block_count_in_chunk = self.block_count_in_chunk.wrapping_add(1);
    }

    pub fn finalize(mut self) -> [u8; OUT_LEN] {
        for b in &mut self.chunk_buf[self.chunk_n..] { *b = 0; }
        self.compress_block(true);
        let mut out = [0u8; OUT_LEN];
        for i in 0..8 {
            out[i*4..i*4+4].copy_from_slice(&self.cv[i].to_le_bytes());
        }
        out
    }
}

pub fn hash(data: &[u8]) -> [u8; OUT_LEN] {
    let mut h = Blake3::new(); h.update(data); h.finalize()
}

const _: () = assert!(BLOCK_LEN == 64 && CHUNK_LEN == 1024);
