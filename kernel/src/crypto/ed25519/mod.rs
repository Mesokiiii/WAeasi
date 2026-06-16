//! Ed25519 — RFC 8032 § 5.1.
pub mod decompress;
pub mod field;
pub mod keypair;
pub mod point;
pub mod scalar;
pub mod tests;

pub use keypair::{keypair, sign, verify, PublicKey, SecretKey, Signature};
pub use tests::self_test;
