//! GDB packet framing — `$<payload>#<csum2>`.
use alloc::string::String;
use alloc::vec::Vec;

/// Read a single packet from `bytes`, return `(payload, consumed)`.
pub fn parse(bytes: &[u8]) -> Option<(String, usize)> {
    let start = bytes.iter().position(|&b| b == b'$')?;
    let hash  = bytes[start + 1..].iter().position(|&b| b == b'#')? + start + 1;
    if hash + 3 > bytes.len() { return None; }
    let payload = core::str::from_utf8(&bytes[start + 1..hash]).ok()?;
    let claimed = u8::from_str_radix(
        core::str::from_utf8(&bytes[hash + 1..hash + 3]).ok()?,
        16
    ).ok()?;
    if checksum(payload.as_bytes()) != claimed { return None; }
    Some((String::from(payload), hash + 3 - start))
}

/// Wrap `payload` into `$payload#csum`.
pub fn frame(payload: &str) -> Vec<u8> {
    let csum = checksum(payload.as_bytes());
    let mut out = Vec::with_capacity(payload.len() + 4);
    out.push(b'$');
    out.extend_from_slice(payload.as_bytes());
    out.push(b'#');
    out.push(hex_nib(csum >> 4));
    out.push(hex_nib(csum & 0x0F));
    out
}

#[inline]
fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |a, &b| a.wrapping_add(b))
}

#[inline]
fn hex_nib(n: u8) -> u8 {
    if n < 10 { b'0' + n } else { b'a' + (n - 10) }
}
