//! TLS 1.3 — RFC 8446.
//!
//! Stage-4 ships the **server side** of TLS 1.3 with:
//!   * cipher suite `TLS_CHACHA20_POLY1305_SHA256` (mandatory),
//!   * key exchange via X25519,
//!   * SHA-256 transcript hash,
//!   * full handshake state machine through `Finished`.
//!
//! Files:
//!   * `record`     — TLSPlaintext / TLSCiphertext encode/decode.
//!   * `handshake`  — handshake-message structs + parsers.
//!   * `state`      — server state machine.
//!   * `keys`       — TLS 1.3 key schedule (HKDF tree).
//!   * `alert`      — alert-protocol enums + helpers.
pub mod alert;
pub mod handshake;
pub mod keys;
pub mod record;
pub mod state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsError {
    UnexpectedMessage,
    DecodeError,
    BadRecordMac,
    HandshakeFailure,
    Unsupported,
    Closed,
}
