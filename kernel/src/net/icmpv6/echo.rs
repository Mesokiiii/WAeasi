//! ICMPv6 Echo Request (Type 128) / Echo Reply (Type 129).
//!
//! Layout (after the 4-byte ICMPv6 common header):
//! ```text
//!   Identifier (2) | Sequence Number (2) | Data (...)
//! ```
use super::header::checksum;
use super::ty;
use crate::net::ipv6::Ipv6Addr;

pub const HEADER_LEN: usize = 4 /*icmpv6*/ + 4 /*id+seq*/;

#[derive(Debug, PartialEq, Eq)]
pub enum EchoError { Short, Oversize, NotEcho, ChecksumMismatch }

const MAX_PAYLOAD: usize = 1500;

/// Try to handle an ICMPv6 Echo Request and emit a reply into `out`.
/// Returns the byte count written.
pub fn handle(
    src: &Ipv6Addr, dst: &Ipv6Addr,
    pkt: &[u8],
    out: &mut [u8],
) -> Result<usize, EchoError> {
    if pkt.len() < HEADER_LEN          { return Err(EchoError::Short); }
    if pkt.len() > HEADER_LEN + MAX_PAYLOAD { return Err(EchoError::Oversize); }
    if pkt[0] != ty::ECHO_REQUEST      { return Err(EchoError::NotEcho); }

    // Verify inbound checksum.
    if checksum(src, dst, pkt) != 0 { return Err(EchoError::ChecksumMismatch); }

    if out.len() < pkt.len() { return Err(EchoError::Short); }
    out[..pkt.len()].copy_from_slice(pkt);
    out[0] = ty::ECHO_REPLY;
    out[2] = 0; out[3] = 0;
    let cs = checksum(dst, src, &out[..pkt.len()]);   // src/dst swapped on reply
    out[2] = (cs >> 8) as u8;
    out[3] = (cs & 0xFF) as u8;
    Ok(pkt.len())
}

/// Build a fresh Echo Request.
pub fn build_request(
    src: &Ipv6Addr, dst: &Ipv6Addr,
    id: u16, seq: u16, data: &[u8],
    out: &mut [u8],
) -> Result<usize, EchoError> {
    let total = HEADER_LEN + data.len();
    if out.len() < total       { return Err(EchoError::Short); }
    if data.len() > MAX_PAYLOAD { return Err(EchoError::Oversize); }
    out[0] = ty::ECHO_REQUEST;
    out[1] = 0;
    out[2] = 0; out[3] = 0;                            // checksum placeholder
    out[4..6].copy_from_slice(&id.to_be_bytes());
    out[6..8].copy_from_slice(&seq.to_be_bytes());
    out[8..total].copy_from_slice(data);
    let cs = checksum(src, dst, &out[..total]);
    out[2] = (cs >> 8) as u8;
    out[3] = (cs & 0xFF) as u8;
    Ok(total)
}
