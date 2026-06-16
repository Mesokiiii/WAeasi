//! Poly1305 — one-time MAC (RFC 8439 § 2.5).
//!
//! All operations are over GF(2^130 - 5).  We follow the canonical
//! 26-bit-limb representation: 5 limbs hold a 130-bit accumulator.
//!
//! Used as the authentication leg of ChaCha20-Poly1305 AEAD.
//! The 32-byte key is **single-use** — derived per AEAD call from
//! ChaCha20's first 32 keystream bytes.
const P: u64 = (1 << 26) - 1;

pub struct Poly1305 {
    r:   [u32; 5],
    s:   [u32; 4],
    acc: [u64; 5],
    buf: [u8; 16],
    buf_len: usize,
}

impl Poly1305 {
    /// `key` is 32 bytes — first 16 are `r` (clamped), last 16 are `s`.
    pub fn new(key: &[u8; 32]) -> Self {
        let mut r_raw = [0u8; 16];
        r_raw.copy_from_slice(&key[..16]);
        // Clamp r per RFC 8439 § 2.5.
        r_raw[3]  &= 15;  r_raw[7]  &= 15;
        r_raw[11] &= 15;  r_raw[15] &= 15;
        r_raw[4]  &= 252; r_raw[8]  &= 252; r_raw[12] &= 252;

        let r0 =  u32::from_le_bytes([r_raw[0],  r_raw[1],  r_raw[2],  r_raw[3]])  & 0x3ff_ffff;
        let r1 = (u32::from_le_bytes([r_raw[3],  r_raw[4],  r_raw[5],  r_raw[6]])  >> 2) & 0x3ff_ff03;
        let r2 = (u32::from_le_bytes([r_raw[6],  r_raw[7],  r_raw[8],  r_raw[9]])  >> 4) & 0x3ff_c0ff;
        let r3 = (u32::from_le_bytes([r_raw[9],  r_raw[10], r_raw[11], r_raw[12]]) >> 6) & 0x3f0_3fff;
        let r4 = (u32::from_le_bytes([r_raw[12], r_raw[13], r_raw[14], r_raw[15]]) >> 8) & 0x00f_ffff;

        let mut s = [0u32; 4];
        for i in 0..4 {
            s[i] = u32::from_le_bytes(key[16 + i*4..16 + i*4 + 4].try_into().unwrap());
        }
        Self { r: [r0, r1, r2, r3, r4], s, acc: [0; 5], buf: [0; 16], buf_len: 0 }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        while !data.is_empty() {
            let need = 16 - self.buf_len;
            let take = need.min(data.len());
            self.buf[self.buf_len..self.buf_len + take].copy_from_slice(&data[..take]);
            self.buf_len += take;
            data = &data[take..];
            if self.buf_len == 16 { self.absorb(true); self.buf_len = 0; }
        }
    }

    pub fn finalize(mut self) -> [u8; 16] {
        if self.buf_len != 0 {
            self.buf[self.buf_len] = 0x01;
            for i in (self.buf_len + 1)..16 { self.buf[i] = 0; }
            self.absorb(false);
        }
        // Reduce + add s.
        let mut h = self.acc;
        carry(&mut h);
        // h += s
        let mut acc: u64 = 0;
        let mut out = [0u8; 16];
        for i in 0..4 {
            let limb = ((h[i] as u64) | ((h[i + 1] as u64) << 26)) >> (i * 6);
            let limb = limb & 0xFFFF_FFFF;
            acc += limb + self.s[i] as u64;
            out[i*4..i*4+4].copy_from_slice(&(acc as u32).to_le_bytes());
            acc >>= 32;
        }
        out
    }

    fn absorb(&mut self, full_block: bool) {
        // Convert 16-byte buf to 5 26-bit limbs + high bit.
        let b = &self.buf;
        let n0 =  u32::from_le_bytes([b[0],  b[1],  b[2],  b[3]])  & 0x3ff_ffff;
        let n1 = (u32::from_le_bytes([b[3],  b[4],  b[5],  b[6]])  >> 2) & 0x3ff_ffff;
        let n2 = (u32::from_le_bytes([b[6],  b[7],  b[8],  b[9]])  >> 4) & 0x3ff_ffff;
        let n3 = (u32::from_le_bytes([b[9],  b[10], b[11], b[12]]) >> 6) & 0x3ff_ffff;
        let n4 = (u32::from_le_bytes([b[12], b[13], b[14], b[15]]) >> 8) | if full_block { 1 << 24 } else { 0 };

        // acc += block
        self.acc[0] += n0 as u64;
        self.acc[1] += n1 as u64;
        self.acc[2] += n2 as u64;
        self.acc[3] += n3 as u64;
        self.acc[4] += n4 as u64;
        multiply(&mut self.acc, &self.r);
    }
}

fn multiply(acc: &mut [u64; 5], r: &[u32; 5]) {
    let mut t = [0u128; 5];
    for i in 0..5 {
        for j in 0..5 {
            let r_j = r[j] as u128 * if i + j > 4 { 5 } else { 1 };
            let idx = (i + j) % 5;
            t[idx] += acc[i] as u128 * r_j;
        }
    }
    for i in 0..5 { acc[i] = (t[i] & ((1 << 26) - 1)) as u64; }
    let mut carry_v = 0u128;
    for i in 0..5 {
        carry_v += t[i] >> 26;
        acc[i] += (carry_v as u64) & P;
        carry_v >>= 26;
    }
    acc[0] += (carry_v as u64) * 5;
}

fn carry(h: &mut [u64; 5]) {
    for i in 0..5 {
        let c = h[i] >> 26;
        h[i] &= 0x3ff_ffff;
        if i < 4 { h[i + 1] += c; } else { h[0] += c * 5; }
    }
}
