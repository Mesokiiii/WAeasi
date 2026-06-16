//! HKDF — HMAC-based Key Derivation (RFC 5869) over SHA-256.
//!
//! Stage-4 perf hardening:
//!   * `expand` builds its T(i) feed in a **stack buffer** of size
//!     `HASH + MAX_INFO + 1`.  No heap allocation on the hot path.
//!   * `expand_label` similarly uses a stack buffer; max info length
//!     = 2 (uint16) + 1 (label_len) + 255 (label) + 1 (ctx_len) + 255 (ctx)
//!     = 514 bytes — safely above any TLS 1.3 use.
//!   * `hmac` continues to use only stack buffers — `BLOCK = 64`.
use super::sha256::Sha256;

const BLOCK: usize = 64;
const HASH:  usize = 32;
const MAX_INFO: usize = 514;

/// HMAC-SHA256.
pub fn hmac(key: &[u8], msg: &[u8]) -> [u8; HASH] {
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        let h = super::sha256::hash(key);
        k[..HASH].copy_from_slice(&h);
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK]; let mut opad = [0x5Cu8; BLOCK];
    for i in 0..BLOCK { ipad[i] ^= k[i]; opad[i] ^= k[i]; }

    let mut inner = Sha256::new();
    inner.update(&ipad); inner.update(msg);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(&opad); outer.update(&inner_digest);
    outer.finalize()
}

/// HKDF-Extract.
pub fn extract(salt: &[u8], ikm: &[u8]) -> [u8; HASH] {
    if salt.is_empty() { hmac(&[0u8; HASH], ikm) }
    else               { hmac(salt, ikm) }
}

/// HKDF-Expand — derive `out.len()` bytes from `prk + info`.  No heap.
pub fn expand(prk: &[u8], info: &[u8], out: &mut [u8]) {
    debug_assert!(info.len() <= MAX_INFO, "info too long for stack buffer");
    let n = (out.len() + HASH - 1) / HASH;
    debug_assert!(n <= 255);

    let mut buf = [0u8; HASH + MAX_INFO + 1];
    let mut prev = [0u8; HASH];
    let mut written = 0;
    for i in 1..=n {
        let mut p = 0;
        if i > 1 { buf[..HASH].copy_from_slice(&prev); p += HASH; }
        buf[p..p + info.len()].copy_from_slice(info); p += info.len();
        buf[p] = i as u8; p += 1;
        prev = hmac(prk, &buf[..p]);

        let take = (out.len() - written).min(HASH);
        out[written..written + take].copy_from_slice(&prev[..take]);
        written += take;
    }
}

/// HKDF-Expand-Label per TLS 1.3 (RFC 8446 § 7.1).  Stack-only.
pub fn expand_label(secret: &[u8], label: &str, context: &[u8], out: &mut [u8]) {
    debug_assert!(label.len() <= 255 - 6 && context.len() <= 255);

    let mut info = [0u8; MAX_INFO];
    let mut p = 0;
    info[p..p + 2].copy_from_slice(&(out.len() as u16).to_be_bytes()); p += 2;
    let full_label_len = 6 + label.len();
    info[p] = full_label_len as u8; p += 1;
    info[p..p + 6].copy_from_slice(b"tls13 "); p += 6;
    info[p..p + label.len()].copy_from_slice(label.as_bytes()); p += label.len();
    info[p] = context.len() as u8; p += 1;
    info[p..p + context.len()].copy_from_slice(context); p += context.len();

    expand(secret, &info[..p], out);
}
