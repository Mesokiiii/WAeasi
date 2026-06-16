//! LEB128 encoding/decoding — unsigned and signed.
use super::ParseError;

const MAX_BYTES_U32: usize = 5;
const MAX_BYTES_U64: usize = 10;

/// Sane upper bound on **count** fields in Wasm sections (number of
/// types, imports, exports, etc.).  A malicious module can encode any
/// `u32`, but no real module has more than a few thousand entries —
/// capping the value used as `Vec::with_capacity` argument prevents
/// memory-exhaustion DoS while still allowing legitimate modules.
pub const MAX_SECTION_COUNT: usize = 100_000;

/// Clamp an attacker-controlled `count` for use as a `Vec` initial
/// capacity.  The actual loop must still bound-check `bytes` so the
/// value can't dictate parser work either.
#[inline]
pub fn safe_capacity(n: u32) -> usize {
    (n as usize).min(MAX_SECTION_COUNT)
}

pub fn read_u32(buf: &[u8], pos: &mut usize) -> Result<u32, ParseError> {
    let mut result: u32 = 0;
    let mut shift = 0;
    for i in 0..MAX_BYTES_U32 {
        let b = *buf.get(*pos).ok_or(ParseError::UnexpectedEof)?;
        *pos += 1;
        result |= ((b & 0x7F) as u32) << shift;
        if b & 0x80 == 0 {
            if i == MAX_BYTES_U32 - 1 && b > 0x0F {
                return Err(ParseError::BadLeb128);
            }
            return Ok(result);
        }
        shift += 7;
    }
    Err(ParseError::BadLeb128)
}

pub fn read_i32(buf: &[u8], pos: &mut usize) -> Result<i32, ParseError> {
    let mut result: i32 = 0;
    let mut shift = 0;
    for i in 0..MAX_BYTES_U32 {
        let b = *buf.get(*pos).ok_or(ParseError::UnexpectedEof)?;
        *pos += 1;
        result |= ((b & 0x7F) as i32) << shift;
        shift += 7;
        if b & 0x80 == 0 {
            if shift < 32 && (b & 0x40) != 0 { result |= !0 << shift; }
            return Ok(result);
        }
        if i == MAX_BYTES_U32 - 1 { return Err(ParseError::BadLeb128); }
    }
    Err(ParseError::BadLeb128)
}

pub fn read_i64(buf: &[u8], pos: &mut usize) -> Result<i64, ParseError> {
    let mut result: i64 = 0;
    let mut shift = 0;
    for i in 0..MAX_BYTES_U64 {
        let b = *buf.get(*pos).ok_or(ParseError::UnexpectedEof)?;
        *pos += 1;
        result |= ((b & 0x7F) as i64) << shift;
        shift += 7;
        if b & 0x80 == 0 {
            if shift < 64 && (b & 0x40) != 0 { result |= !0 << shift; }
            return Ok(result);
        }
        if i == MAX_BYTES_U64 - 1 { return Err(ParseError::BadLeb128); }
    }
    Err(ParseError::BadLeb128)
}
