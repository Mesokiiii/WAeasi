//! ICMPv6 error messages — Type 1..4.
//!
//! All four share the layout:
//! ```text
//!   Type (1) | Code (1) | Checksum (2) | <Type-specific 4 bytes> | Original Datagram (...)
//! ```
//!
//! `Original Datagram` is "as much of the offending packet as fits into
//! the minimum IPv6 MTU (1280 bytes) without exceeding it."
use super::header::checksum;
use super::ty;
use crate::net::ipv6::Ipv6Addr;

pub const HEADER_LEN: usize = 8;
pub const MIN_MTU:    usize = 1280;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DstUnreachCode {
    NoRoute        = 0,
    AdminProhibit  = 1,
    BeyondScope    = 2,
    AddressUnreach = 3,
    PortUnreach    = 4,
    SrcAddrFailedPolicy = 5,
    RejectRoute    = 6,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TimeExceededCode { HopLimit = 0, FragReassembly = 1 }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ParamProblemCode {
    BadHeader      = 0,
    UnknownNextHdr = 1,
    UnknownOption  = 2,
}

/// Build a generic error message.  `type_specific` is 4 bytes (e.g. MTU
/// for Packet-Too-Big, pointer for Param-Problem, or zeros).
pub fn build_into(
    out: &mut [u8],
    src: &Ipv6Addr, dst: &Ipv6Addr,
    kind: u8, code: u8,
    type_specific: [u8; 4],
    original: &[u8],
) -> Option<usize> {
    let body_max = MIN_MTU - 40 /* IPv6 hdr */ - HEADER_LEN;
    let body_len = original.len().min(body_max);
    let total    = HEADER_LEN + body_len;
    if out.len() < total { return None; }

    out[0] = kind;
    out[1] = code;
    out[2] = 0; out[3] = 0;                         // checksum placeholder
    out[4..8].copy_from_slice(&type_specific);
    out[8..total].copy_from_slice(&original[..body_len]);

    let cs = checksum(src, dst, &out[..total]);
    out[2] = (cs >> 8) as u8;
    out[3] = (cs & 0xFF) as u8;
    Some(total)
}

#[inline]
pub fn dest_unreach(out: &mut [u8], src: &Ipv6Addr, dst: &Ipv6Addr,
                    code: DstUnreachCode, original: &[u8]) -> Option<usize> {
    build_into(out, src, dst, ty::DST_UNREACH, code as u8, [0; 4], original)
}

#[inline]
pub fn packet_too_big(out: &mut [u8], src: &Ipv6Addr, dst: &Ipv6Addr,
                      mtu: u32, original: &[u8]) -> Option<usize> {
    build_into(out, src, dst, ty::PACKET_TOO_BIG, 0, mtu.to_be_bytes(), original)
}

#[inline]
pub fn time_exceeded(out: &mut [u8], src: &Ipv6Addr, dst: &Ipv6Addr,
                     code: TimeExceededCode, original: &[u8]) -> Option<usize> {
    build_into(out, src, dst, ty::TIME_EXCEEDED, code as u8, [0; 4], original)
}
