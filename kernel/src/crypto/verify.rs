//! Module-binary verification.
//!
//! Stage 2 supports two integrity checks that components must pass before
//! the engine compiles their bytecode:
//!
//!   * **SHA-256 manifest match** — compare hash to a kernel-baked
//!     allowlist (compiled in for stage 2; replaced by a Wasm-component
//!     trust store in stage 4).
//!   * **Magic + version** — sanity gate identical to wasm-spec.
//!
//! Returning `Err` aborts loading; the caller logs an audit entry.
use super::sha256::Sha256;

#[derive(Debug)]
pub enum VerifyError {
    BadMagic,
    BadVersion,
    HashMismatch,
    NotInAllowlist,
}

#[derive(Copy, Clone, Debug)]
pub struct AllowEntry {
    pub name:   &'static str,
    pub digest: [u8; 32],
}

/// Compile-time trust store.  Stage 2 hardcodes; stage 4 will replace
/// with a signed manifest file loaded by the boot service.
pub const ALLOWLIST: &[AllowEntry] = &[];

/// Run all checks against `bytes`, with `name` used for log + match.
pub fn verify_module(name: &str, bytes: &[u8]) -> Result<[u8; 32], VerifyError> {
    if bytes.len() < 8 || &bytes[..4] != b"\0asm" {
        return Err(VerifyError::BadMagic);
    }
    let v = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if v != 1 { return Err(VerifyError::BadVersion); }

    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();

    if ALLOWLIST.is_empty() {
        // Empty allowlist → development mode: log, accept.
        log::warn!("[verify] '{}' loaded without allowlist match (DEV MODE)", name);
        return Ok(digest);
    }
    for entry in ALLOWLIST {
        if entry.name == name {
            if constant_time_eq32(&entry.digest, &digest) {
                return Ok(digest);
            } else {
                return Err(VerifyError::HashMismatch);
            }
        }
    }
    Err(VerifyError::NotInAllowlist)
}

/// Constant-time 32-byte compare.
fn constant_time_eq32(a: &[u8; 32], b: &[u8; 32]) -> bool {
    let mut diff: u8 = 0;
    for i in 0..32 { diff |= a[i] ^ b[i]; }
    diff == 0
}
