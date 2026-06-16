//! ChaCha20-Poly1305 AEAD (RFC 8439).
//!
//! Combines ChaCha20 keystream with a Poly1305 MAC over (AAD || pad ||
//! ciphertext || pad || aad_len || ct_len).  This is the cipher used
//! by TLS 1.3's mandatory `TLS_CHACHA20_POLY1305_SHA256`.
//!
//! Stage-4 implementation focuses on **constant-time** operation on the
//! tag-compare path: any timing leak in `verify` defeats the auth.
use super::chacha20::ChaCha20;
use super::poly1305::Poly1305;

/// 256-bit key + 96-bit nonce (RFC 8439 § 2.8).
pub struct Aead {
    key:   [u8; 32],
    nonce: [u8; 12],
}

impl Aead {
    pub fn new(key: [u8; 32], nonce: [u8; 12]) -> Self { Self { key, nonce } }

    /// Encrypt `plaintext` in place; return the 16-byte tag.
    pub fn seal(&self, aad: &[u8], plaintext: &mut [u8]) -> [u8; 16] {
        let mut keystream_block0 = [0u8; 64];
        ChaCha20::new(&self.key, &self.nonce).fill(&mut keystream_block0);
        let mut poly_key = [0u8; 32];
        poly_key.copy_from_slice(&keystream_block0[..32]);

        // Encrypt with counter starting at 1 (block 0 was used for poly key).
        let mut cipher = ChaCha20::new(&self.key, &self.nonce);
        let mut burn = [0u8; 64];
        cipher.fill(&mut burn);                   // discard block 0
        xor_keystream(&mut cipher, plaintext);

        let tag = compute_tag(&poly_key, aad, plaintext);
        tag
    }

    /// Verify `tag` and decrypt `ciphertext` in place.  Returns `Err`
    /// on tag mismatch — `ciphertext` is then **scrambled** (do not use).
    pub fn open(&self, aad: &[u8], ciphertext: &mut [u8], tag: &[u8; 16]) -> Result<(), ()> {
        let mut keystream_block0 = [0u8; 64];
        ChaCha20::new(&self.key, &self.nonce).fill(&mut keystream_block0);
        let mut poly_key = [0u8; 32];
        poly_key.copy_from_slice(&keystream_block0[..32]);

        let expected = compute_tag(&poly_key, aad, ciphertext);
        if !ct_eq(&expected, tag) { return Err(()); }

        let mut cipher = ChaCha20::new(&self.key, &self.nonce);
        let mut burn = [0u8; 64];
        cipher.fill(&mut burn);
        xor_keystream(&mut cipher, ciphertext);
        Ok(())
    }
}

fn xor_keystream(cipher: &mut ChaCha20, data: &mut [u8]) {
    let mut buf = [0u8; 64];
    let mut i = 0;
    while i < data.len() {
        cipher.fill(&mut buf);
        let take = (data.len() - i).min(64);
        for j in 0..take { data[i + j] ^= buf[j]; }
        i += take;
    }
}

fn compute_tag(poly_key: &[u8; 32], aad: &[u8], ct: &[u8]) -> [u8; 16] {
    let mut p = Poly1305::new(poly_key);
    p.update(aad);
    pad16(&mut p, aad.len());
    p.update(ct);
    pad16(&mut p, ct.len());
    let mut lens = [0u8; 16];
    lens[..8].copy_from_slice(&(aad.len() as u64).to_le_bytes());
    lens[8..].copy_from_slice(&(ct.len()  as u64).to_le_bytes());
    p.update(&lens);
    p.finalize()
}

fn pad16(p: &mut Poly1305, n: usize) {
    let pad = (16 - (n % 16)) % 16;
    if pad != 0 { p.update(&[0u8; 16][..pad]); }
}

#[inline]
fn ct_eq(a: &[u8; 16], b: &[u8; 16]) -> bool {
    let mut diff: u8 = 0;
    for i in 0..16 { diff |= a[i] ^ b[i]; }
    diff == 0
}
