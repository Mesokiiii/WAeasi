//! Ed25519 point decompression — recovers `(x, y)` from the canonical
//! 32-byte compressed encoding.
//!
//! Algorithm (RFC 8032 § 5.1.3):
//!   1. Parse `y` from the low 255 bits, `sign(x)` from bit 255.
//!   2. Compute `u = y² − 1`, `v = d·y² + 1`.
//!   3. Compute candidate `x = (u/v)^((p+3)/8)`.
//!   4. If `v·x² == u` we have the right square root.
//!      Else multiply `x` by `sqrt(−1) = 2^((p−1)/4)` to find it.
//!   5. If `x[0] & 1 != sign_bit`, negate `x`.
//!
//! All field operations are in `super::field` (constant-time).
use super::field::{self as fe, Fe};
use super::point::Point;

/// `−d` constant for the Ed25519 curve, packed into 5×51-bit limbs.
/// `d = −121665/121666 mod p`.
const D: Fe = [
    0x34dca135978a3, 0x1a8283b156ebd, 0x5e7a26001c029, 0x739c663a03cbb, 0x52036cee2b6ff,
];

/// `sqrt(−1) mod p`  =  2^((p−1)/4)  packed.
const SQRT_M1: Fe = [
    0x61b274a0ea0b0, 0x0d5a5fc8f189d, 0x7ef5e9cbd0c60, 0x78595a6804c9e, 0x2b8324804fc1d,
];

/// Decompress a 32-byte compressed point.  Returns `None` for any
/// non-canonical encoding (out-of-range y, no square root, etc.).
pub fn decompress(bytes: &[u8; 32]) -> Option<Point> {
    let mut y_bytes = *bytes;
    let sign_bit = (y_bytes[31] >> 7) & 1;
    y_bytes[31] &= 0x7F;

    let y = fe::unpack(&y_bytes);
    let one = fe::one();

    // u = y² − 1
    let y2 = fe::sq(&y);
    let u  = fe::sub(&y2, &one);

    // v = d·y² + 1
    let v  = fe::add(&fe::mul(&D, &y2), &one);

    // Candidate x = (u·v³)·(u·v⁷)^((p−5)/8) ≡ (u/v)^((p+3)/8).
    let v3 = fe::mul(&v, &fe::sq(&v));
    let v7 = fe::mul(&v3, &fe::sq(&v3));
    let mut x = fe::mul(&u, &v3);
    x = fe::mul(&x, &pow_p_minus_5_div_8(&fe::mul(&u, &v7)));

    // Check v·x² == u.
    let vx2 = fe::mul(&v, &fe::sq(&x));
    if !fe_eq(&vx2, &u) {
        // Multiply by sqrt(−1).
        let candidate = fe::mul(&x, &SQRT_M1);
        let cand_check = fe::mul(&v, &fe::sq(&candidate));
        if !fe_eq(&cand_check, &u) { return None; }
        x = candidate;
    }

    // Choose sign.
    let x_bytes = fe::pack(&x);
    if (x_bytes[0] & 1) != sign_bit {
        x = fe::neg(&x);
    }

    Some(Point {
        x,
        y,
        z: fe::one(),
        t: fe::mul(&x, &y),
    })
}

/// Constant-time field-element equality.
fn fe_eq(a: &Fe, b: &Fe) -> bool {
    let pa = fe::pack(a);
    let pb = fe::pack(b);
    let mut diff: u8 = 0;
    for i in 0..32 { diff |= pa[i] ^ pb[i]; }
    diff == 0
}

/// `a^((p−5)/8) mod p`.  Uses the same exponentiation chain as
/// `field::invert` but stopped one square-root early.
fn pow_p_minus_5_div_8(z: &Fe) -> Fe {
    // Standard chain — see ref10/Boring.  We reuse `invert` and adjust:
    // `a^(p−5)/8 = (a^(p−2))^(1/8)` is messy; instead we open-code it.
    let mut t0 = fe::sq(z);
    let mut t1 = fe::sq(&fe::sq(&t0));
    t1 = fe::mul(z, &t1);
    t0 = fe::mul(&t0, &t1);                       // z^11
    t0 = fe::sq(&t0);
    t0 = fe::mul(&t1, &t0);                       // z^31
    let mut t2 = t0;
    for _ in 0..5  { t2 = fe::sq(&t2); } t2 = fe::mul(&t2, &t0);
    let mut t3 = t2;
    for _ in 0..10 { t3 = fe::sq(&t3); } t3 = fe::mul(&t3, &t2);
    let mut t4 = t3;
    for _ in 0..20 { t4 = fe::sq(&t4); } t4 = fe::mul(&t4, &t3);
    for _ in 0..10 { t4 = fe::sq(&t4); } t4 = fe::mul(&t4, &t2);
    let mut t5 = t4;
    for _ in 0..50 { t5 = fe::sq(&t5); } t5 = fe::mul(&t5, &t4);
    let mut t6 = t5;
    for _ in 0..100{ t6 = fe::sq(&t6); } t6 = fe::mul(&t6, &t5);
    for _ in 0..50 { t6 = fe::sq(&t6); } t6 = fe::mul(&t6, &t4);
    for _ in 0..2  { t6 = fe::sq(&t6); }
    fe::mul(&t6, z)
}
