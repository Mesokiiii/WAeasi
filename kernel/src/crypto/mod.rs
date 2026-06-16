//! Cryptographic primitives.
//!
//! Stage-9 inventory (2026-baseline):
//!
//! Hashes:
//!   * `sha256` / `sha512`   — FIPS 180-4 (legacy compatibility).
//!   * `sha3` (256/384/512)  — FIPS 202 (modern, PQ-foundation).
//!   * `shake` (128/256)     — FIPS 202 XOF, ML-KEM/ML-DSA building block.
//!   * `blake3`              — fast tree-hash (RFC-style spec, 2020).
//!
//! Symmetric:
//!   * `chacha20`            — RFC 8439 stream.
//!   * `poly1305`            — RFC 8439 one-time MAC.
//!   * `chacha20poly1305`    — RFC 8439 AEAD (TLS 1.3 mandatory).
//!   * `aes` (128/256 + GCM) — NIST FIPS 197 + SP 800-38D.  AES-NI hook.
//!
//! Asymmetric:
//!   * `x25519`              — RFC 7748 ECDH (classical).
//!   * `ed25519`             — RFC 8032 sign / verify (classical).
//!   * `ml_kem` (768)        — FIPS 203 post-quantum KEM.
//!
//! Higher-level:
//!   * `hkdf`                — RFC 5869 + RFC 8446 expand-label.
//!   * `hmac`                — generic HMAC<H>; SHA-256/SHA-512 instances.
//!   * `asn1` + `x509`       — DER reader + certificate parsing + chain.
//!   * `tls`                 — TLS 1.3 record + handshake.
//!
//! Cross-cutting:
//!   * `ct`                  — constant-time comparison primitives.
//!   * `verify`              — module-binary signature gate.
pub mod aes;
pub mod argon2;
pub mod asn1;
pub mod blake3;
pub mod chacha20;
pub mod chacha20poly1305;
pub mod ct;
pub mod ed25519;
pub mod hkdf;
pub mod hmac;
pub mod keccak;
pub mod ml_dsa;
pub mod ml_kem;
pub mod poly1305;
pub mod sha256;
pub mod sha3;
pub mod sha512;
pub mod shake;
pub mod tls;
pub mod verify;
pub mod x25519;
pub mod x509;
