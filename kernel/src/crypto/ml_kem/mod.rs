//! ML-KEM (Module-Lattice-based Key-Encapsulation) — FIPS 203.
//!
//! Stage-10 production tier:
//!   * `params`  — FIPS 203 constants for {512, 768, 1024}.
//!   * `poly`    — `R_q = Z_q[X]/(X^256 + 1)` polynomial arithmetic.
//!   * `sample`  — CBD sampling + uniform-from-XOF for matrix A.
//!   * `kpke`    — K-PKE.KeyGen / Encrypt / Decrypt (the inner crypto).
//!   * `encaps`  — Full Fujisaki-Okamoto encapsulation entry.
//!   * `decaps`  — Decapsulation with constant-time implicit rejection.
//!
//! Default profile: ML-KEM-768 (NIST Level 3, TLS 1.3 hybrid baseline).
pub mod decaps;
pub mod encaps;
pub mod kpke;
pub mod params;
pub mod poly;
pub mod sample;

pub use kpke::{keygen, KPkeKeys};
pub use params::{MlKemLevel, MLKEM_512, MLKEM_768, MLKEM_1024};

#[derive(Clone)]
pub struct PublicKey { pub bytes: alloc::vec::Vec<u8>, pub level: MlKemLevel }

pub struct SecretKey { pub bytes: alloc::vec::Vec<u8>, pub level: MlKemLevel }

impl Drop for SecretKey {
    fn drop(&mut self) {
        for b in self.bytes.iter_mut() {
            unsafe { core::ptr::write_volatile(b as *mut u8, 0); }
        }
    }
}

#[derive(Clone)]
pub struct Ciphertext { pub bytes: alloc::vec::Vec<u8>, pub level: MlKemLevel }

pub type SharedSecret = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KemError { BadLength, InternalFault }
