//! Distribution sampling for ML-KEM.
//!
//! Two sampling routines per FIPS 203:
//!   * **CBD** — centered binomial distribution η, used to sample
//!     "small" polynomials s, e, r, e1, e2.
//!   * **Uniform-from-XOF** — sample a uniformly-random polynomial in
//!     `R_q` from a SHAKE128 stream (used to derive matrix `A`).
//!
//! Both are constant-time over the secret bits.
use crate::crypto::shake::Shake128;
use super::poly::{Poly, N, Q};

/// Centered Binomial Distribution sampling.  `eta` ∈ {2, 3}.
/// Output is a polynomial whose coefficients lie in `[-eta, eta]`.
///
/// For each coefficient: gather `2η` random bits; let `a` = sum of
/// first `η` bits, `b` = sum of next `η` bits; coeff = a - b.
pub fn cbd(seed: &[u8], eta: u8) -> Poly {
    let need_bytes = (N * eta as usize * 2) / 8;
    let mut buf = alloc::vec![0u8; need_bytes];
    Shake128::new()
        .with_input(seed)
        .squeeze_into(&mut buf);
    let mut p = Poly::zero();
    let mut bit_pos = 0usize;
    for i in 0..N {
        let mut a = 0i16;
        let mut b = 0i16;
        for _ in 0..eta { a += bit_at(&buf, bit_pos) as i16; bit_pos += 1; }
        for _ in 0..eta { b += bit_at(&buf, bit_pos) as i16; bit_pos += 1; }
        // Result is in [-eta, +eta]; map to [0, q).
        let coeff = a - b;
        p.coeffs[i] = if coeff < 0 { coeff + Q as i16 } else { coeff };
    }
    p
}

#[inline]
fn bit_at(buf: &[u8], pos: usize) -> u8 {
    (buf[pos / 8] >> (pos & 7)) & 1
}

/// Sample a uniformly-random polynomial in `R_q` from a SHAKE128 stream.
/// Rejection-sampled: read 3 bytes → 2 candidate 12-bit values; accept
/// each iff `< q`.
pub fn uniform(seed: &[u8], i: u8, j: u8) -> Poly {
    let mut input = alloc::vec::Vec::with_capacity(seed.len() + 2);
    input.extend_from_slice(seed);
    input.push(j);
    input.push(i);
    let mut shake = Shake128::new();
    shake.update(&input);

    let mut p = Poly::zero();
    let mut filled = 0;
    let mut chunk = [0u8; 168];     // SHAKE128 rate
    while filled < N {
        shake.squeeze_into(&mut chunk);
        let mut k = 0;
        while k + 3 <= chunk.len() && filled < N {
            let d1 = ((chunk[k]     as u16))      | (((chunk[k+1] as u16) & 0x0F) << 8);
            let d2 = ((chunk[k+1] as u16) >> 4) |  ((chunk[k+2] as u16)        << 4);
            if (d1 as i32) < Q { p.coeffs[filled] = d1 as i16; filled += 1; }
            if filled < N && (d2 as i32) < Q {
                p.coeffs[filled] = d2 as i16; filled += 1;
            }
            k += 3;
        }
    }
    p
}

/// Helper for the SHAKE128 builder pattern used above.
trait ShakeBuilder {
    fn with_input(self, data: &[u8]) -> Self;
}
impl ShakeBuilder for Shake128 {
    fn with_input(mut self, data: &[u8]) -> Self {
        self.update(data);
        self
    }
}
