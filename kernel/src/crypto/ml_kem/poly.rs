//! Polynomial arithmetic in `R_q = Z_q[X] / (X^256 + 1)`, q = 3329.
//!
//! Used by ML-KEM (Kyber) per FIPS 203.  Stage-10 implements:
//!   * Coefficient representation as `[i16; 256]` (signed for easy Sub).
//!   * Schoolbook multiplication (correct, O(n²) — clean foundation).
//!   * Negacyclic reduction `X^n = -1 mod (X^n + 1)`.
//!   * Barrett reduction for q = 3329.
//!
//! Stage-11 will replace schoolbook with NTT (negacyclic 256-point);
//! for now schoolbook keeps the stage reviewable.
pub const N: usize = 256;
pub const Q: i32   = 3329;

#[derive(Copy, Clone)]
pub struct Poly { pub coeffs: [i16; N] }

impl Poly {
    pub const fn zero() -> Self { Self { coeffs: [0; N] } }

    /// Coefficient-wise addition mod q.
    pub fn add(&self, other: &Self) -> Self {
        let mut o = Self::zero();
        for i in 0..N { o.coeffs[i] = barrett(self.coeffs[i] as i32 + other.coeffs[i] as i32); }
        o
    }

    /// Coefficient-wise subtraction mod q.
    pub fn sub(&self, other: &Self) -> Self {
        let mut o = Self::zero();
        for i in 0..N { o.coeffs[i] = barrett(self.coeffs[i] as i32 - other.coeffs[i] as i32); }
        o
    }

    /// Schoolbook polynomial multiplication mod (X^n + 1, q).
    pub fn mul(&self, other: &Self) -> Self {
        let mut acc = [0i32; N];
        for i in 0..N {
            for j in 0..N {
                let prod = (self.coeffs[i] as i32) * (other.coeffs[j] as i32);
                let k = i + j;
                if k < N {
                    acc[k] += prod;
                } else {
                    // Negacyclic: X^N = -1
                    acc[k - N] -= prod;
                }
            }
            // Periodic reduction to keep accumulators in i32 range.
            if i % 8 == 7 {
                for c in &mut acc { *c = barrett(*c) as i32; }
            }
        }
        let mut o = Self::zero();
        for i in 0..N { o.coeffs[i] = barrett(acc[i]); }
        o
    }

    /// Pack coefficients into 12-bit-per-coeff byte stream (384 bytes).
    pub fn to_bytes_12(&self) -> [u8; 384] {
        let mut out = [0u8; 384];
        for i in 0..(N / 2) {
            let a = self.coeffs[2 * i]     as u16 & 0xFFF;
            let b = self.coeffs[2 * i + 1] as u16 & 0xFFF;
            let off = i * 3;
            out[off]     =  a as u8;
            out[off + 1] = ((a >> 8) | (b << 4)) as u8;
            out[off + 2] = (b >> 4) as u8;
        }
        out
    }

    pub fn from_bytes_12(bytes: &[u8; 384]) -> Self {
        let mut p = Self::zero();
        for i in 0..(N / 2) {
            let off = i * 3;
            let a = (bytes[off]     as u16) | (((bytes[off+1] as u16) & 0x0F) << 8);
            let b = ((bytes[off+1] as u16) >> 4) | ((bytes[off+2] as u16) << 4);
            p.coeffs[2 * i]     = (a & 0xFFF) as i16;
            p.coeffs[2 * i + 1] = (b & 0xFFF) as i16;
        }
        p
    }
}

/// Barrett reduction modulo q = 3329.  Output is in `[0, q)` after
/// adjustment.  Branch-free.
#[inline]
pub fn barrett(a: i32) -> i16 {
    // q = 3329, m = floor(2^26 / q) = 20159
    const M: i32 = 20159;
    let t = ((a as i64 * M as i64) >> 26) as i32;
    let r = a - t * Q;
    // Map to [0, q).
    let r = if r < 0 { r + Q } else if r >= Q { r - Q } else { r };
    r as i16
}

/// Vec<Poly> — module element of length `k`.
pub type PolyVec = alloc::vec::Vec<Poly>;

pub fn polyvec_add(a: &PolyVec, b: &PolyVec) -> PolyVec {
    debug_assert_eq!(a.len(), b.len());
    a.iter().zip(b.iter()).map(|(x, y)| x.add(y)).collect()
}

/// Inner product `sum(a_i * b_i)` over the module.
pub fn polyvec_inner(a: &PolyVec, b: &PolyVec) -> Poly {
    debug_assert_eq!(a.len(), b.len());
    let mut acc = Poly::zero();
    for i in 0..a.len() {
        let prod = a[i].mul(&b[i]);
        acc = acc.add(&prod);
    }
    acc
}
