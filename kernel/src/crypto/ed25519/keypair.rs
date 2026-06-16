//! Ed25519 sign / verify (RFC 8032 § 5.1).
//!
//! Procedure (sign):
//!   1. `h     = SHA-512(secret_key)`
//!   2. `a     = clamp(h[..32])`            // private scalar
//!   3. `prefix = h[32..]`
//!   4. `A     = a · B`                     // public key
//!   5. `r     = SHA-512(prefix ‖ msg) mod l`
//!   6. `R     = r · B`
//!   7. `k     = SHA-512(R ‖ A ‖ msg) mod l`
//!   8. `S     = (r + k · a) mod l`
//!   9. signature = `R ‖ S`
//!
//! Verify checks that `S · B == R + k · A`.
use super::point::{self, scalar_mul, Point};
use super::scalar;
use crate::crypto::sha512;

pub type SecretKey = [u8; 32];
pub type PublicKey = [u8; 32];
pub type Signature = [u8; 64];

/// Derive a public key from a secret key (the seed).
pub fn keypair(seed: &SecretKey) -> PublicKey {
    let h = sha512::hash(seed);
    let a = clamp(&h[..32].try_into().unwrap());
    let A = scalar_mul(&a, &Point::basepoint());
    point::compress(&A)
}

/// Sign `msg` with the seed.  Returns 64-byte signature `R ‖ S`.
pub fn sign(seed: &SecretKey, msg: &[u8]) -> Signature {
    let h = sha512::hash(seed);
    let a = clamp(&h[..32].try_into().unwrap());
    let prefix: [u8; 32] = h[32..].try_into().unwrap();

    // r = SHA-512(prefix ‖ msg) mod l
    let mut r_hasher = sha512::Sha512::new();
    r_hasher.update(&prefix);
    r_hasher.update(msg);
    let r_full = r_hasher.finalize();
    let r = scalar::reduce(&r_full);

    // R = r · B,  encoded.
    let R = point::compress(&scalar_mul(&r, &Point::basepoint()));

    // A = a · B,  encoded.
    let A = point::compress(&scalar_mul(&a, &Point::basepoint()));

    // k = SHA-512(R ‖ A ‖ msg) mod l
    let mut k_hasher = sha512::Sha512::new();
    k_hasher.update(&R); k_hasher.update(&A); k_hasher.update(msg);
    let k = scalar::reduce(&k_hasher.finalize());

    // S = (r + k · a) mod l
    let S = scalar::add_mul(&r, &k, &a);

    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&R);
    sig[32..].copy_from_slice(&S);
    sig
}

/// Verify `signature` against `public_key` and `msg`.
pub fn verify(public_key: &PublicKey, msg: &[u8], signature: &Signature) -> bool {
    let R = &signature[..32];
    let S: [u8; 32] = signature[32..].try_into().unwrap();

    // S < l guard (constant-time-ish — comparison is a public check).
    if !s_in_range(&S) { return false; }

    // k = SHA-512(R ‖ A ‖ msg) mod l
    let mut k_hasher = sha512::Sha512::new();
    k_hasher.update(R);
    k_hasher.update(public_key);
    k_hasher.update(msg);
    let k = scalar::reduce(&k_hasher.finalize());

    // Stage-5 verification: compute S·B - k·A and compare to R.
    // Skipping the negation algebra here — we do `S·B == R + k·A` via
    // recompression of both sides.
    let SB = scalar_mul(&S, &Point::basepoint());
    let R_decoded = match decompress(R.try_into().unwrap()) {
        Some(p) => p, None => return false,
    };
    let A_decoded = match decompress(public_key) {
        Some(p) => p, None => return false,
    };
    let kA = scalar_mul(&k, &A_decoded);
    let rhs = point::add(&R_decoded, &kA);

    // Compare compressed forms — constant-time over the public bytes.
    constant_time_eq32(&point::compress(&SB), &point::compress(&rhs))
}

fn clamp(h: &[u8; 32]) -> [u8; 32] {
    let mut a = *h;
    a[0]  &= 248;
    a[31] &= 127;
    a[31] |= 64;
    a
}

fn s_in_range(s: &[u8; 32]) -> bool {
    // Spec lower bound: s < l.  Uses the same byte compare as scalar::ge.
    const L_LE: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58,
        0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
    ];
    for i in (0..32).rev() {
        if s[i] != L_LE[i] { return s[i] < L_LE[i]; }
    }
    false
}

fn constant_time_eq32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff: u8 = 0;
    for i in 0..32 { diff |= a[i] ^ b[i]; }
    diff == 0
}

/// Edwards-form decompression — recover (X, Y, Z, T) from a 32-byte
/// compressed encoding.  Delegates to `decompress::decompress`, which
/// performs the canonical sqrt-recovery on the curve equation.
fn decompress(compressed: &[u8; 32]) -> Option<Point> {
    super::decompress::decompress(compressed)
}
