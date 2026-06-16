//! ICMPv6 common 4-byte header + pseudo-header checksum.
//!
//! Pseudo-header layout (RFC 8200 § 8.1):
//! ```text
//!   Source Address (16) | Dest Address (16) | Length (4 BE) | Zero (3) | NextHdr=58 (1)
//! ```
use crate::net::ipv6::Ipv6Addr;

#[derive(Copy, Clone, Debug)]
pub struct IcmpV6Header {
    pub kind:     u8,
    pub code:     u8,
    pub checksum: u16,
}

#[derive(Copy, Clone, Debug)]
pub struct IcmpV6Type(pub u8);

pub const HEADER_LEN: usize = 4;

#[inline]
fn fold(mut s: u32) -> u16 {
    while s >> 16 != 0 { s = (s & 0xFFFF) + (s >> 16); }
    !(s as u16)
}

/// Compute the ICMPv6 checksum over the (pseudo-header + body).
/// `body` is the entire ICMPv6 packet (including its header) with the
/// checksum field zeroed.
pub fn checksum(src: &Ipv6Addr, dst: &Ipv6Addr, body: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    // Pseudo-header.
    for chunk in src.0.chunks(2) { sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32; }
    for chunk in dst.0.chunks(2) { sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32; }
    let len = body.len() as u32;
    sum += (len >> 16) & 0xFFFF;
    sum += len & 0xFFFF;
    sum += 58;                                    // Next Header = ICMPv6
    // Body.
    let mut i = 0;
    while i + 1 < body.len() {
        sum += u16::from_be_bytes([body[i], body[i+1]]) as u32;
        i += 2;
    }
    if i < body.len() { sum += (body[i] as u32) << 8; }
    fold(sum)
}

pub fn parse(buf: &[u8]) -> Option<IcmpV6Header> {
    if buf.len() < HEADER_LEN { return None; }
    Some(IcmpV6Header {
        kind:     buf[0],
        code:     buf[1],
        checksum: u16::from_be_bytes([buf[2], buf[3]]),
    })
}
