//! AES-128 / AES-256 + AES-GCM AEAD.
pub mod aesni;
pub mod encrypt;
pub mod gcm;
pub mod key;

pub use encrypt::{encrypt_block, AesBlock};
pub use gcm::{Aes128Gcm, Aes256Gcm, GcmError};
pub use key::{KeySchedule, Aes128Key, Aes256Key};
