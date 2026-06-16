//! sha-256 helpers + Component Model magic verification.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{BuildError, Result};

const CHUNK: usize = 64 * 1024;
const COMPONENT_MAGIC: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00];
const CORE_MAGIC: [u8; 8]      = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];

/// Streaming sha-256 over `path`.  Returns lowercase hex digest.
pub fn file(path: &Path) -> Result<String> {
    let mut f = File::open(path).map_err(|e| BuildError::io(path, e))?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; CHUNK];
    loop {
        let n = f.read(&mut buf).map_err(|e| BuildError::io(path, e))?;
        if n == 0 { break; }
        h.update(&buf[..n]);
    }
    Ok(hex_lower(&h.finalize()))
}

/// sha-256 over an in-memory slice.
pub fn bytes(data: &[u8]) -> [u8; 32] {
    let h = Sha256::digest(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h);
    out
}

/// Detect whether `path` is a Wasm Component (vs a core module).
///
/// Returns:
///   * `Ok(true)`  — Component Model layered binary
///   * `Ok(false)` — core module (also valid input; will be wrapped)
///   * `Err(...)`  — file too small or unrecognisable header
pub fn is_component(path: &Path) -> Result<bool> {
    let mut f = File::open(path).map_err(|e| BuildError::io(path, e))?;
    let mut head = [0u8; 8];
    f.read_exact(&mut head)
        .map_err(|_| BuildError::BadComponent(
            format!("{}: file too small for wasm header", path.display()),
        ))?;
    if head == COMPONENT_MAGIC {
        Ok(true)
    } else if head == CORE_MAGIC {
        Ok(false)
    } else {
        Err(BuildError::BadComponent(
            format!("{}: not a wasm file (bad magic)", path.display()),
        ))
    }
}

/// Lowercase hex helper.  Implemented locally to avoid a `hex` crate dep.
pub fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

/// Decode lowercase hex back to bytes.
pub fn hex_decode(s: &str) -> Result<Vec<u8>> {
    if s.len() % 2 != 0 {
        return Err(BuildError::Bundle(format!("odd-length hex string: {} chars", s.len())));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = nibble(bytes[i])?;
        let lo = nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn nibble(b: u8) -> Result<u8> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(BuildError::Bundle(format!("invalid hex byte: 0x{b:02x}"))),
    }
}
