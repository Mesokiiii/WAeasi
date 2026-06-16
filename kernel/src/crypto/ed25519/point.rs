//! Edwards-curve point arithmetic.
//!
//! Curve: `-x² + y² = 1 + d·x²·y²` over GF(2^255 - 19), where
//! `d = -121665/121666`.
//!
//! Extended Edwards coordinates `(X, Y, Z, T)` give complete addition
//! formulas (no special cases for doubling / mixed addition) — exactly
//! what we want for **constant-time** scalar multiplication.
use super::field::{self as fe, Fe};

#[derive(Copy, Clone, Debug)]
pub struct Point { pub x: Fe, pub y: Fe, pub z: Fe, pub t: Fe }

impl Point {
    pub fn identity() -> Self {
        Self { x: fe::zero(), y: fe::one(), z: fe::one(), t: fe::zero() }
    }

    /// Affine basepoint B for Ed25519, lifted into extended coordinates.
    /// `By` constant from RFC 8032 § 5.1; `Bx` is recovered from the
    /// curve equation given `By`.  Both are pre-baked here.
    pub fn basepoint() -> Self {
        // Bx hex: 216936D3CD6E53FEC0A4E231FDD6DC5C692CC7609525A7B2C9562D608F25D51A
        // By hex: 6666666666666666666666666666666666666666666666666666666666666658
        let bx_bytes: [u8; 32] = hex_le("1ad5258fd62d56c9b2a72595606cc2925cdcd6fd31e2a4c0fe536ecdd3366921");
        let by_bytes: [u8; 32] = hex_le("5866666666666666666666666666666666666666666666666666666666666666");
        let x = fe::unpack(&bx_bytes);
        let y = fe::unpack(&by_bytes);
        Self { x, y, z: fe::one(), t: fe::mul(&x, &y) }
    }
}

/// Twisted-Edwards addition (a = -1) using extended coordinates.
pub fn add(p: &Point, q: &Point) -> Point {
    let a = fe::mul(&fe::sub(&p.y, &p.x), &fe::sub(&q.y, &q.x));
    let b = fe::mul(&fe::add(&p.y, &p.x), &fe::add(&q.y, &q.x));
    let c = fe::mul(&fe::add(&p.t, &p.t), &fe::mul(&q.t, &D2));
    let d = fe::mul(&fe::add(&p.z, &p.z), &q.z);
    let e = fe::sub(&b, &a);
    let f = fe::sub(&d, &c);
    let g = fe::add(&d, &c);
    let h = fe::add(&b, &a);
    Point {
        x: fe::mul(&e, &f),
        y: fe::mul(&g, &h),
        z: fe::mul(&f, &g),
        t: fe::mul(&e, &h),
    }
}

pub fn double(p: &Point) -> Point { add(p, p) }

/// Constant-time scalar multiplication via straightforward double-and-add
/// with a constant-time `cswap` (no early exit on bit value).
pub fn scalar_mul(scalar: &[u8; 32], p: &Point) -> Point {
    let mut q = Point::identity();
    let mut r = *p;
    for byte_idx in 0..32 {
        for bit_idx in 0..8 {
            let bit = ((scalar[byte_idx] >> bit_idx) & 1) as u64;
            let mut sum = add(&q, &r);
            // cswap (q, sum) on bit
            cswap_point(bit, &mut q, &mut sum);
            r = double(&r);
        }
    }
    q
}

fn cswap_point(swap: u64, a: &mut Point, b: &mut Point) {
    fe::cswap(swap, &mut a.x, &mut b.x);
    fe::cswap(swap, &mut a.y, &mut b.y);
    fe::cswap(swap, &mut a.z, &mut b.z);
    fe::cswap(swap, &mut a.t, &mut b.t);
}

/// Curve constant `2 * d` precomputed.  Stage 5 stores it as a packed
/// field element.
const D2: Fe = [
    0x69b9426b2f159, 0x35050762add7a, 0x3cf44c0038052, 0x6738cc7407977, 0x2406d9dc56dff,
];

/// Decode a 64-character hex string into 32 little-endian bytes.
const fn hex_le(s: &str) -> [u8; 32] {
    let bytes = s.as_bytes();
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        let hi = hex_nib(bytes[i*2]);
        let lo = hex_nib(bytes[i*2 + 1]);
        out[i] = (hi << 4) | lo;
        i += 1;
    }
    out
}

const fn hex_nib(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

/// Compress an extended-coordinates point into the canonical 32-byte
/// little-endian form (sign bit of x packed into the MSB of y).
pub fn compress(p: &Point) -> [u8; 32] {
    let zinv = fe::invert(&p.z);
    let x = fe::mul(&p.x, &zinv);
    let y = fe::mul(&p.y, &zinv);
    let mut out = fe::pack(&y);
    let x_bytes = fe::pack(&x);
    out[31] |= (x_bytes[0] & 1) << 7;
    out
}
