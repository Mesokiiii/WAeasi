//! ML-KEM encapsulation (FIPS 203 § 7.2) — full Fujisaki-Okamoto-K
//! transform on top of `kpke::encrypt`.
use crate::crypto::sha3::{sha3_256, sha3_512};

use super::kpke;
use super::params::{Params, MLKEM_768};
use super::{Ciphertext, KemError, PublicKey, SharedSecret};

pub fn encapsulate(pk: &PublicKey) -> Result<(Ciphertext, SharedSecret), KemError> {
    encapsulate_with(pk, &MLKEM_768)
}

pub fn encapsulate_with(pk: &PublicKey, params: &Params)
    -> Result<(Ciphertext, SharedSecret), KemError>
{
    if pk.bytes.len() != params.pk_len { return Err(KemError::BadLength); }

    // 1. Sample uniform 32-byte message m.
    let mut m = [0u8; 32];
    let r_bytes = crate::wasi::preview2::random::get_random_bytes(32);
    m.copy_from_slice(&r_bytes);

    // 2. (K_bar, r) = G(m ‖ H(pk))
    let h_pk = sha3_256(&pk.bytes);
    let mut g_input = alloc::vec::Vec::with_capacity(64);
    g_input.extend_from_slice(&m);
    g_input.extend_from_slice(&h_pk);
    let g_out = sha3_512(&g_input);
    let mut k_bar = [0u8; 32];
    let mut r_seed = [0u8; 32];
    k_bar.copy_from_slice(&g_out[..32]);
    r_seed.copy_from_slice(&g_out[32..]);

    // 3. c = K-PKE.Encrypt(pk, m, r) — real math.
    let ct_bytes = kpke::encrypt(&pk.bytes, &m, &r_seed, params);

    // 4. K = SHA3-256(K_bar ‖ H(c))   (FIPS 203 derivation J).
    let h_ct = sha3_256(&ct_bytes);
    let mut k_input = alloc::vec::Vec::with_capacity(64);
    k_input.extend_from_slice(&k_bar);
    k_input.extend_from_slice(&h_ct);
    let mut shared = [0u8; 32];
    shared.copy_from_slice(&sha3_256(&k_input));

    Ok((Ciphertext { bytes: ct_bytes, level: params.level }, shared))
}
