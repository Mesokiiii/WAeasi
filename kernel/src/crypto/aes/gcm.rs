//! AES-GCM AEAD (NIST SP 800-38D).
//!
//! GCM = AES-CTR encryption + GHASH authentication over GF(2^128).
//! Stage-9 software path; CLMUL hardware acceleration lands in stage-10
//! once `cpuid::CLMUL` is plumbed through.
//!
//! Used by TLS 1.3 cipher suites:
//!   * `TLS_AES_128_GCM_SHA256`
//!   * `TLS_AES_256_GCM_SHA384`
use super::encrypt::{ctr_block, encrypt_block, AesBlock};
use super::key::{expand_128, expand_256, KeySchedule};
use crate::crypto::ct;

#[derive(Debug, PartialEq, Eq)]
pub enum GcmError { TagMismatch, BadNonce }

pub struct Aes128Gcm { ks: KeySchedule, h: AesBlock }
pub struct Aes256Gcm { ks: KeySchedule, h: AesBlock }

impl Aes128Gcm {
    pub fn new(key: &[u8; 16]) -> Self {
        let ks = expand_128(key);
        let mut h = [0u8; 16];
        encrypt_block(&ks, &mut h);
        Self { ks, h }
    }
    pub fn seal(&self, nonce: &[u8; 12], aad: &[u8], plaintext: &mut [u8]) -> [u8; 16] {
        seal(&self.ks, &self.h, nonce, aad, plaintext)
    }
    pub fn open(&self, nonce: &[u8; 12], aad: &[u8], ct: &mut [u8], tag: &[u8; 16])
        -> Result<(), GcmError>
    {
        open(&self.ks, &self.h, nonce, aad, ct, tag)
    }
}

impl Aes256Gcm {
    pub fn new(key: &[u8; 32]) -> Self {
        let ks = expand_256(key);
        let mut h = [0u8; 16];
        encrypt_block(&ks, &mut h);
        Self { ks, h }
    }
    pub fn seal(&self, nonce: &[u8; 12], aad: &[u8], plaintext: &mut [u8]) -> [u8; 16] {
        seal(&self.ks, &self.h, nonce, aad, plaintext)
    }
    pub fn open(&self, nonce: &[u8; 12], aad: &[u8], ct: &mut [u8], tag: &[u8; 16])
        -> Result<(), GcmError>
    {
        open(&self.ks, &self.h, nonce, aad, ct, tag)
    }
}

fn seal(ks: &KeySchedule, h: &AesBlock, nonce: &[u8; 12],
        aad: &[u8], pt: &mut [u8]) -> [u8; 16] {
    // J0 = nonce || 0x00000001
    let mut j0 = [0u8; 16];
    j0[..12].copy_from_slice(nonce);
    j0[15] = 1;

    // Encrypt plaintext using CTR starting from inc(J0).
    ctr_xor(ks, &j0, pt);

    // GHASH over (AAD ‖ pad || CT ‖ pad ‖ aad_len(64) || ct_len(64)).
    let tag = ghash_tag(h, ks, &j0, aad, pt);
    tag
}

fn open(ks: &KeySchedule, h: &AesBlock, nonce: &[u8; 12],
        aad: &[u8], ct: &mut [u8], tag: &[u8; 16]) -> Result<(), GcmError> {
    let mut j0 = [0u8; 16];
    j0[..12].copy_from_slice(nonce);
    j0[15] = 1;

    let expected = ghash_tag(h, ks, &j0, aad, ct);
    if !ct::eq_bytes(&expected, tag) { return Err(GcmError::TagMismatch); }

    ctr_xor(ks, &j0, ct);
    Ok(())
}

fn ctr_xor(ks: &KeySchedule, j0: &AesBlock, data: &mut [u8]) {
    let mut counter = *j0;
    let mut block = [0u8; 16];
    let mut i = 0;
    while i < data.len() {
        inc_be32(&mut counter);
        ctr_block(ks, &counter, &mut block);
        let take = (data.len() - i).min(16);
        for j in 0..take { data[i + j] ^= block[j]; }
        i += take;
    }
}

fn inc_be32(b: &mut AesBlock) {
    let v = u32::from_be_bytes([b[12], b[13], b[14], b[15]]).wrapping_add(1);
    b[12..16].copy_from_slice(&v.to_be_bytes());
}

fn ghash_tag(h: &AesBlock, ks: &KeySchedule, j0: &AesBlock,
             aad: &[u8], ct: &[u8]) -> [u8; 16] {
    let mut y = [0u8; 16];
    ghash_blocks(h, &mut y, aad);
    ghash_blocks(h, &mut y, ct);
    let mut len_block = [0u8; 16];
    len_block[0..8].copy_from_slice(&((aad.len() as u64) * 8).to_be_bytes());
    len_block[8..16].copy_from_slice(&((ct.len()  as u64) * 8).to_be_bytes());
    for i in 0..16 { y[i] ^= len_block[i]; }
    gf128_mul(&mut y, h);

    // Encrypt J0, XOR with Y → tag.
    let mut s = *j0;
    encrypt_block(ks, &mut s);
    for i in 0..16 { y[i] ^= s[i]; }
    y
}

fn ghash_blocks(h: &AesBlock, y: &mut AesBlock, data: &[u8]) {
    let mut i = 0;
    while i < data.len() {
        let take = (data.len() - i).min(16);
        let mut block = [0u8; 16];
        block[..take].copy_from_slice(&data[i..i + take]);
        for j in 0..16 { y[j] ^= block[j]; }
        gf128_mul(y, h);
        i += take;
    }
}

/// GF(2^128) multiplication — NIST SP 800-38D shift-and-add.  CLMUL
/// hardware path (one `pclmulqdq`) lands stage-10.
fn gf128_mul(x: &mut AesBlock, y: &AesBlock) {
    let mut z = [0u8; 16];
    let mut v = *y;
    for i in 0..128 {
        let bit = (x[i / 8] >> (7 - (i & 7))) & 1;
        if bit == 1 {
            for j in 0..16 { z[j] ^= v[j]; }
        }
        let lsb = v[15] & 1;
        // Right-shift v by 1, MSB-first.
        let mut carry = 0u8;
        for j in 0..16 {
            let new_carry = (v[j] & 1) << 7;
            v[j] = (v[j] >> 1) | carry;
            carry = new_carry;
        }
        if lsb == 1 { v[0] ^= 0xE1; }
    }
    *x = z;
}
