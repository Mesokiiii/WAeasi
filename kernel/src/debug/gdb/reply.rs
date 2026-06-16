//! GDB-side reply formatting helpers.
use alloc::string::String;
use core::fmt::Write;

/// `OK` reply — used by every command that succeeds without payload.
pub fn ok() -> String { String::from("OK") }

/// `ENN` error reply (NN is hex).
pub fn err(code: u8) -> String {
    let mut s = String::with_capacity(3);
    s.push('E');
    let _ = write!(&mut s, "{:02x}", code);
    s
}

/// Stop reply — `Sxx` for a signal.  GDB expects `S05` for SIGTRAP.
pub fn stop(signal: u8) -> String {
    let mut s = String::with_capacity(3);
    s.push('S');
    let _ = write!(&mut s, "{:02x}", signal);
    s
}

/// Encode a buffer as hex (each byte → two lowercase hex chars).
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}
