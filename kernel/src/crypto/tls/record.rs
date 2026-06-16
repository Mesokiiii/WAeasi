//! TLS 1.3 record layer (RFC 8446 § 5).
//!
//! TLSPlaintext:
//! ```text
//!   ContentType  type        // 1 byte
//!   uint16       legacy_version    = 0x0303
//!   uint16       length            // ≤ 2^14
//!   opaque       fragment[length]
//! ```
//!
//! After `Finished`, every record is a TLSCiphertext: outer
//! `application_data` ContentType, inner `ContentType + padding` is
//! AEAD-protected.
use super::TlsError;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ContentType {
    ChangeCipherSpec = 20,
    Alert            = 21,
    Handshake        = 22,
    ApplicationData  = 23,
}

impl ContentType {
    pub fn from_u8(b: u8) -> Result<Self, TlsError> {
        match b {
            20 => Ok(Self::ChangeCipherSpec),
            21 => Ok(Self::Alert),
            22 => Ok(Self::Handshake),
            23 => Ok(Self::ApplicationData),
            _  => Err(TlsError::DecodeError),
        }
    }
}

pub const HEADER_LEN:    usize = 5;
pub const MAX_PLAINTEXT: usize = 16384;          // 2^14
pub const MAX_CIPHERTEXT: usize = MAX_PLAINTEXT + 256;

#[derive(Debug)]
pub struct Record<'a> {
    pub content: ContentType,
    pub fragment: &'a [u8],
}

pub fn parse(buf: &[u8]) -> Result<(Record<'_>, usize), TlsError> {
    if buf.len() < HEADER_LEN { return Err(TlsError::DecodeError); }
    let content = ContentType::from_u8(buf[0])?;
    let _legacy = u16::from_be_bytes([buf[1], buf[2]]);
    let len = u16::from_be_bytes([buf[3], buf[4]]) as usize;
    if len > MAX_CIPHERTEXT { return Err(TlsError::DecodeError); }
    if buf.len() < HEADER_LEN + len { return Err(TlsError::DecodeError); }
    Ok((Record { content, fragment: &buf[HEADER_LEN..HEADER_LEN + len] },
        HEADER_LEN + len))
}

/// Build a TLSPlaintext record into `out`.  Returns the byte count written.
pub fn build(out: &mut [u8], content: ContentType, fragment: &[u8]) -> Result<usize, TlsError> {
    let total = HEADER_LEN + fragment.len();
    if out.len() < total { return Err(TlsError::DecodeError); }
    out[0] = content as u8;
    out[1] = 0x03; out[2] = 0x03;
    out[3] = (fragment.len() >> 8) as u8;
    out[4] = (fragment.len() & 0xFF) as u8;
    out[HEADER_LEN..total].copy_from_slice(fragment);
    Ok(total)
}
