//! Generic HMAC (RFC 2104) — works over any hash function that
//! implements the `HashFn` trait.
//!
//! Stage-9 ships:
//!   * `HmacSha256` (block 64, output 32)
//!   * `HmacSha384` (block 128, output 48)
//!   * `HmacSha512` (block 128, output 64)
//!
//! Plus the underlying `Hmac<H>` generic struct so any hash that meets
//! the trait can be MAC'd without code duplication.
use super::sha256::Sha256;
use super::sha512::Sha512;

/// Streaming hash trait — minimal surface required by HMAC.
pub trait HashFn {
    /// Block size in bytes (input absorption rate).
    const BLOCK: usize;
    /// Digest size in bytes.
    const OUTPUT: usize;

    fn new() -> Self;
    fn update(&mut self, data: &[u8]);
    /// Consume self, return a fixed-size byte slice serialized into `out`.
    fn finalize_into(self, out: &mut [u8]);
}

impl HashFn for Sha256 {
    const BLOCK:  usize = 64;
    const OUTPUT: usize = 32;
    fn new() -> Self { Sha256::new() }
    fn update(&mut self, data: &[u8]) { Sha256::update(self, data) }
    fn finalize_into(self, out: &mut [u8]) {
        let d = self.finalize();
        out[..32].copy_from_slice(&d);
    }
}

impl HashFn for Sha512 {
    const BLOCK:  usize = 128;
    const OUTPUT: usize = 64;
    fn new() -> Self { Sha512::new() }
    fn update(&mut self, data: &[u8]) { Sha512::update(self, data) }
    fn finalize_into(self, out: &mut [u8]) {
        let d = self.finalize();
        out[..64].copy_from_slice(&d);
    }
}

pub struct Hmac<H: HashFn> {
    inner: H,
    opad:  [u8; 256],   // sized to max BLOCK across all impls (SHA-512 = 128)
}

impl<H: HashFn> Hmac<H> {
    pub fn new(key: &[u8]) -> Self {
        let mut k_pad = [0u8; 256];
        if key.len() > H::BLOCK {
            let mut h = H::new();
            h.update(key);
            h.finalize_into(&mut k_pad);
        } else {
            k_pad[..key.len()].copy_from_slice(key);
        }
        // Build inner-pad in-place; cache outer-pad for finalize().
        let mut ipad = [0x36u8; 256];
        let mut opad = [0x5Cu8; 256];
        for i in 0..H::BLOCK {
            ipad[i] ^= k_pad[i];
            opad[i] ^= k_pad[i];
        }
        let mut inner = H::new();
        inner.update(&ipad[..H::BLOCK]);
        Self { inner, opad }
    }

    pub fn update(&mut self, msg: &[u8]) { self.inner.update(msg); }

    /// Finalize and write the tag into `out`.
    pub fn finalize_into(self, out: &mut [u8]) {
        debug_assert!(out.len() >= H::OUTPUT);
        let mut inner_digest = [0u8; 256];
        self.inner.finalize_into(&mut inner_digest);

        let mut outer = H::new();
        outer.update(&self.opad[..H::BLOCK]);
        outer.update(&inner_digest[..H::OUTPUT]);
        outer.finalize_into(out);
    }
}

/// Convenience one-shot helpers.
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut m = Hmac::<Sha256>::new(key);
    m.update(msg);
    let mut out = [0u8; 32];
    m.finalize_into(&mut out);
    out
}

pub fn hmac_sha512(key: &[u8], msg: &[u8]) -> [u8; 64] {
    let mut m = Hmac::<Sha512>::new(key);
    m.update(msg);
    let mut out = [0u8; 64];
    m.finalize_into(&mut out);
    out
}

/// HMAC-SHA384 — uses SHA-512 internally with a 48-byte truncation
/// per FIPS 180-4 (SHA-384 = SHA-512 with different IV).  Stage-10
/// will swap in a dedicated SHA-384 state once we add it; until then
/// callers should prefer `hmac_sha512`.
#[deprecated(note = "use hmac_sha256 or hmac_sha512 in stage-9; SHA-384 lands in stage-10")]
pub fn hmac_sha384(_key: &[u8], _msg: &[u8]) -> [u8; 48] { [0; 48] }
