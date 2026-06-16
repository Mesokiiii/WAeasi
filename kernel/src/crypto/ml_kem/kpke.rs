//! K-PKE (the inner public-key encryption underlying ML-KEM).
//!
//! Per FIPS 203 § 6.  This is the **functional** crypto layer — `encaps`/
//! `decaps` wrap it with the Fujisaki-Okamoto transform for IND-CCA2.
//!
//! All three operations (`keygen`, `encrypt`, `decrypt`) operate on
//! the polynomials defined in `super::poly` and the sampling routines
//! in `super::sample`.
use alloc::vec::Vec;

use crate::crypto::sha3::sha3_256;

use super::params::Params;
use super::poly::{polyvec_inner, Poly, PolyVec};
use super::sample::{cbd, uniform};

/// KeyGen output: public-key bytes (matrix-row encoding) + secret-key bytes.
pub struct KPkeKeys { pub pk: Vec<u8>, pub sk: Vec<u8> }

/// `K-PKE.KeyGen(d)` — derives both keys from a 32-byte seed.
pub fn keygen(seed_d: &[u8; 32], params: &Params) -> KPkeKeys {
    let g = sha3_256(seed_d);                  // (ρ, σ) split below
    let rho   = &g[..16];
    let sigma = &g[16..];

    // Sample matrix A ∈ R_q^{k×k} from ρ.
    let mut a_matrix: Vec<Vec<Poly>> = Vec::with_capacity(params.k);
    for i in 0..params.k {
        let mut row = Vec::with_capacity(params.k);
        for j in 0..params.k { row.push(uniform(rho, i as u8, j as u8)); }
        a_matrix.push(row);
    }

    // Sample s, e ∈ R_q^k from σ via CBD_eta1.
    let s = sample_polyvec(sigma, params.k, params.eta1, 0);
    let e = sample_polyvec(sigma, params.k, params.eta1, params.k as u8);

    // t = A·s + e
    let mut t: PolyVec = Vec::with_capacity(params.k);
    for i in 0..params.k {
        let mut row_dot = polyvec_inner(&a_matrix[i], &s);
        row_dot = row_dot.add(&e[i]);
        t.push(row_dot);
    }

    let pk = encode_pk(&t, rho);
    let sk = encode_sk(&s);
    KPkeKeys { pk, sk }
}

/// `K-PKE.Encrypt(pk, m, r)` — encrypts the 32-byte message `m`.
pub fn encrypt(pk: &[u8], message: &[u8; 32], rand_seed: &[u8; 32], params: &Params) -> Vec<u8> {
    let (t, rho) = decode_pk(pk, params.k);

    // Re-derive matrix A from ρ.
    let mut a_matrix: Vec<Vec<Poly>> = Vec::with_capacity(params.k);
    for i in 0..params.k {
        let mut row = Vec::with_capacity(params.k);
        for j in 0..params.k { row.push(uniform(&rho, i as u8, j as u8)); }
        a_matrix.push(row);
    }

    // Sample r ∈ R_q^k via CBD_eta1, e1 ∈ R_q^k via CBD_eta2, e2 ∈ R_q via CBD_eta2.
    let r_vec = sample_polyvec(rand_seed, params.k, params.eta1, 0);
    let e1    = sample_polyvec(rand_seed, params.k, params.eta2, params.k as u8);
    let e2    = sample_polyvec(rand_seed, 1, params.eta2, (2 * params.k) as u8)[0];

    // u = A^T · r + e1
    let mut u: PolyVec = Vec::with_capacity(params.k);
    for j in 0..params.k {
        let col: Vec<Poly> = (0..params.k).map(|i| a_matrix[i][j]).collect();
        let mut dot = polyvec_inner(&col, &r_vec);
        dot = dot.add(&e1[j]);
        u.push(dot);
    }

    // v = t^T · r + e2 + Decompress(m)
    let mut v = polyvec_inner(&t, &r_vec);
    v = v.add(&e2);
    v = v.add(&decode_message(message));

    let mut ct = Vec::with_capacity(params.ct_len);
    for poly in &u { ct.extend_from_slice(&poly.to_bytes_12()); }
    ct.extend_from_slice(&v.to_bytes_12());
    ct
}

/// `K-PKE.Decrypt(sk, c)` — recovers the 32-byte message.
pub fn decrypt(sk: &[u8], ct: &[u8], params: &Params) -> [u8; 32] {
    let s = decode_sk(sk, params.k);
    let (u, v) = decode_ct(ct, params.k);
    let mut m_poly = v.sub(&polyvec_inner(&s, &u));
    encode_message(&mut m_poly)
}

// ─── Encoding helpers ──────────────────────────────────────────────

fn encode_pk(t: &PolyVec, rho: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(t.len() * 384 + rho.len());
    for poly in t { out.extend_from_slice(&poly.to_bytes_12()); }
    out.extend_from_slice(rho);
    out
}

fn decode_pk(bytes: &[u8], k: usize) -> (PolyVec, Vec<u8>) {
    let mut t = Vec::with_capacity(k);
    for i in 0..k {
        let off = i * 384;
        let mut buf = [0u8; 384];
        buf.copy_from_slice(&bytes[off..off + 384]);
        t.push(Poly::from_bytes_12(&buf));
    }
    let rho = bytes[k * 384..].to_vec();
    (t, rho)
}

fn encode_sk(s: &PolyVec) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 384);
    for poly in s { out.extend_from_slice(&poly.to_bytes_12()); }
    out
}

fn decode_sk(bytes: &[u8], k: usize) -> PolyVec {
    (0..k).map(|i| {
        let off = i * 384;
        let mut buf = [0u8; 384];
        buf.copy_from_slice(&bytes[off..off + 384]);
        Poly::from_bytes_12(&buf)
    }).collect()
}

fn decode_ct(bytes: &[u8], k: usize) -> (PolyVec, Poly) {
    let u: PolyVec = (0..k).map(|i| {
        let off = i * 384;
        let mut buf = [0u8; 384];
        buf.copy_from_slice(&bytes[off..off + 384]);
        Poly::from_bytes_12(&buf)
    }).collect();
    let v_off = k * 384;
    let mut buf = [0u8; 384];
    buf.copy_from_slice(&bytes[v_off..v_off + 384]);
    (u, Poly::from_bytes_12(&buf))
}

fn decode_message(m: &[u8; 32]) -> Poly {
    let mut p = Poly::zero();
    for i in 0..32 {
        for j in 0..8 {
            let bit = (m[i] >> j) & 1;
            p.coeffs[i * 8 + j] = if bit == 1 { ((super::poly::Q + 1) / 2) as i16 } else { 0 };
        }
    }
    p
}

fn encode_message(p: &mut Poly) -> [u8; 32] {
    let mut m = [0u8; 32];
    for i in 0..32 {
        for j in 0..8 {
            let c = p.coeffs[i * 8 + j];
            // Threshold-decode: 1 iff coefficient near q/2.
            let close_to_half = (c - ((super::poly::Q + 1) / 2) as i16).abs() < (super::poly::Q as i16 / 4);
            if close_to_half { m[i] |= 1 << j; }
        }
    }
    m
}

fn sample_polyvec(seed: &[u8], k: usize, eta: u8, base_nonce: u8) -> PolyVec {
    (0..k).map(|i| {
        let mut s = alloc::vec::Vec::with_capacity(seed.len() + 1);
        s.extend_from_slice(seed);
        s.push(base_nonce + i as u8);
        cbd(&s, eta)
    }).collect()
}
