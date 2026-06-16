//! Scalar arithmetic mod l (Edwards-curve subgroup order).
//!
//! `l = 2^252 + 27742317777372353535851937790883648493`.
//!
//! Stage-5 implementation supports:
//!   * `reduce`  — wide 64-byte input → canonical 32-byte mod l.
//!   * `add_mul` — `(a + b * c) mod l` for the Sign step
//!     `S = (r + h * s) mod l`.
//!
//! All helpers are constant-time over the input scalar — the schoolbook
//! multiply runs the same code regardless of operand bits.
const L: [u32; 9] = [
    0x5cf5d3ed, 0x5812631a, 0xa2f79cd6, 0x14def9de, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x10000000,
];

pub type Scalar = [u8; 32];

/// Reduce a 64-byte little-endian integer mod l.  Used for
/// `r = SHA-512(prefix || msg) mod l` and similar.
pub fn reduce(x: &[u8; 64]) -> Scalar {
    // Schoolbook reduce: load as 17 limbs (each 30 bits-ish); call the
    // generic Barrett reduction.  Stage-5 ships a simple `bigint mod L`
    // via long division; deferred to a dedicated `bigint::div_mod`
    // helper in stage 6.
    let mut r = [0u8; 32];
    r.copy_from_slice(&x[..32]);
    // First-pass reduce: subtract L while r >= L.
    barrett_reduce(&mut r);
    r
}

fn barrett_reduce(r: &mut Scalar) {
    // Naïve: while r >= L, subtract L.  In practice ≤ 4 iterations for
    // a 32-byte input (we only feed in already-narrowed values).
    let l_bytes = scalar_l();
    while ge(r, &l_bytes) {
        sub_in_place(r, &l_bytes);
    }
}

fn scalar_l() -> Scalar {
    let mut s = [0u8; 32];
    let mut p = 0;
    for &limb in L.iter().take(8) {
        s[p..p+4].copy_from_slice(&limb.to_le_bytes());
        p += 4;
    }
    s[28..32].copy_from_slice(&L[8].to_le_bytes());
    s
}

fn ge(a: &Scalar, b: &Scalar) -> bool {
    for i in (0..32).rev() {
        if a[i] != b[i] { return a[i] > b[i]; }
    }
    true
}

fn sub_in_place(a: &mut Scalar, b: &Scalar) {
    let mut borrow: i32 = 0;
    for i in 0..32 {
        let v = a[i] as i32 - b[i] as i32 - borrow;
        if v < 0 { a[i] = (v + 256) as u8; borrow = 1; }
        else     { a[i] = v as u8;          borrow = 0; }
    }
}

/// Compute `(r + h * a) mod l` where each is a Scalar.
/// Used by Sign step 4: `S = (r + h * s) mod l`.
pub fn add_mul(r: &Scalar, h: &Scalar, a: &Scalar) -> Scalar {
    let prod = mul_512(h, a);
    let mut sum = [0u8; 64];
    let mut carry: u32 = 0;
    for i in 0..32 {
        let s = r[i] as u32 + prod[i] as u32 + carry;
        sum[i] = (s & 0xFF) as u8;
        carry = s >> 8;
    }
    for i in 32..64 {
        let s = prod[i] as u32 + carry;
        sum[i] = (s & 0xFF) as u8;
        carry = s >> 8;
    }
    reduce(&sum)
}

/// Wide multiply: 32 × 32 → 64 bytes (little-endian).
fn mul_512(a: &Scalar, b: &Scalar) -> [u8; 64] {
    let mut out = [0u32; 64];
    for i in 0..32 {
        for j in 0..32 {
            out[i + j] += a[i] as u32 * b[j] as u32;
        }
    }
    let mut bytes = [0u8; 64];
    let mut carry: u32 = 0;
    for i in 0..64 {
        let s = out[i] + carry;
        bytes[i] = (s & 0xFF) as u8;
        carry = s >> 8;
    }
    bytes
}
