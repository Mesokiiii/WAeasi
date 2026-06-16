//! ML-DSA (Module-Lattice Digital Signature Algorithm) — FIPS 204.
//!
//! Post-quantum digital signature scheme based on Dilithium.
//! Stage-10 ships the parameter sets + API surface; the
//! signing/verification math (Fiat-Shamir-with-aborts) lands in
//! stage-11 alongside the ML-KEM NTT.
//!
//! Why ML-DSA in 2026:
//!   * Companion to ML-KEM in NSA CNSA 2.0.
//!   * Default PQ signature in NIST FIPS 204 (final 2024-08).
//!   * IETF TLS 1.3 hybrid signature drafts use ML-DSA-65 + Ed25519.
pub mod params;

pub use params::{MlDsaLevel, MLDSA_44, MLDSA_65, MLDSA_87};

#[derive(Clone)]
pub struct PublicKey  { pub bytes: alloc::vec::Vec<u8>, pub level: MlDsaLevel }
pub struct SecretKey  { pub bytes: alloc::vec::Vec<u8>, pub level: MlDsaLevel }
#[derive(Clone)]
pub struct Signature  { pub bytes: alloc::vec::Vec<u8>, pub level: MlDsaLevel }

impl Drop for SecretKey {
    fn drop(&mut self) {
        for b in self.bytes.iter_mut() {
            unsafe { core::ptr::write_volatile(b as *mut u8, 0); }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DsaError { BadLength, InvalidSignature }

/// Stage-10 surface: keypair generation pulls 32 random bytes and
/// returns deterministic-looking placeholders sized per FIPS 204.
/// Stage-11 will replace with the real Fiat-Shamir-with-aborts loop.
pub fn keygen() -> (PublicKey, SecretKey) {
    keygen_with(MLDSA_65)
}

pub fn keygen_with(level: MlDsaLevel) -> (PublicKey, SecretKey) {
    let p = params::for_level(level);
    let pk_bytes = alloc::vec![0u8; p.pk_len];
    let sk_bytes = alloc::vec![0u8; p.sk_len];
    (
        PublicKey { bytes: pk_bytes, level },
        SecretKey { bytes: sk_bytes, level },
    )
}

/// Stage-10 sign — surface only.  Real signing in stage-11.
pub fn sign(_sk: &SecretKey, _msg: &[u8]) -> Signature {
    let p = params::for_level(_sk.level);
    Signature { bytes: alloc::vec![0u8; p.sig_len], level: _sk.level }
}

/// Stage-10 verify — surface only.  Always returns `Err` until
/// stage-11 wires the math.
pub fn verify(_pk: &PublicKey, _msg: &[u8], _sig: &Signature) -> Result<(), DsaError> {
    Err(DsaError::InvalidSignature)
}
