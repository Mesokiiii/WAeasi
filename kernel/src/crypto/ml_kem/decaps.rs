//! ML-KEM decapsulation (FIPS 203 § 7.3) with implicit rejection.
use crate::crypto::ct;
use crate::crypto::sha3::{sha3_256, sha3_512};

use super::kpke;
use super::params::{Params, MLKEM_768};
use super::{Ciphertext, KemError, SecretKey, SharedSecret};

pub fn decapsulate(sk: &SecretKey, ct: &Ciphertext) -> Result<SharedSecret, KemError> {
    decapsulate_with(sk, ct, &MLKEM_768)
}

pub fn decapsulate_with(sk: &SecretKey, ct: &Ciphertext, params: &Params)
    -> Result<SharedSecret, KemError>
{
    if ct.bytes.len() != params.ct_len { return Err(KemError::BadLength); }

    // ML-KEM secret-key layout (FIPS 203):  sk = sk' ‖ pk ‖ H(pk) ‖ z
    // For stage-10 we accept the simpler layout: sk = sk' ‖ pk ‖ z.
    // `sk'` is `kpke` secret; `z` is the implicit-rejection seed.
    let sk_inner_len = 384 * params.k;
    if sk.bytes.len() < sk_inner_len + params.pk_len + 32 {
        return Err(KemError::BadLength);
    }
    let sk_inner = &sk.bytes[..sk_inner_len];
    let pk_bytes = &sk.bytes[sk_inner_len..sk_inner_len + params.pk_len];
    let z        = &sk.bytes[sk_inner_len + params.pk_len ..
                              sk_inner_len + params.pk_len + 32];

    // 1. m' = K-PKE.Decrypt(sk', c)
    let m_prime = kpke::decrypt(sk_inner, &ct.bytes, params);

    // 2. (K'_bar, r') = G(m' ‖ H(pk))
    let h_pk = sha3_256(pk_bytes);
    let mut g_input = alloc::vec::Vec::with_capacity(64);
    g_input.extend_from_slice(&m_prime);
    g_input.extend_from_slice(&h_pk);
    let g_out = sha3_512(&g_input);
    let mut k_bar = [0u8; 32];
    let mut r_seed = [0u8; 32];
    k_bar.copy_from_slice(&g_out[..32]);
    r_seed.copy_from_slice(&g_out[32..]);

    // 3. c' = K-PKE.Encrypt(pk, m', r')
    let ct_prime = kpke::encrypt(pk_bytes, &m_prime, &r_seed, params);
    let valid = ct::eq_bytes(&ct_prime, &ct.bytes);

    // 4. Derive both branches; constant-time select.
    let h_ct = sha3_256(&ct.bytes);

    let mut k_ok_input = alloc::vec::Vec::with_capacity(64);
    k_ok_input.extend_from_slice(&k_bar);
    k_ok_input.extend_from_slice(&h_ct);
    let k_ok = sha3_256(&k_ok_input);

    let mut k_rej_input = alloc::vec::Vec::with_capacity(32 + ct.bytes.len());
    k_rej_input.extend_from_slice(z);
    k_rej_input.extend_from_slice(&ct.bytes);
    let k_rej = sha3_256(&k_rej_input);

    let mut out = [0u8; 32];
    for i in 0..32 {
        let mask = (valid as u8).wrapping_neg();
        out[i] = (mask & k_ok[i]) | (!mask & k_rej[i]);
    }
    Ok(out)
}
