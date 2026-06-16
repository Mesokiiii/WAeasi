//! TCP/IP checksum — supports both IPv4 (RFC 9293) and IPv6 (RFC 8200 §8.1).
//!
//! Pseudo-headers:
//!
//! IPv4 (12 bytes):
//! ```text
//!   src_addr (4)  dst_addr (4)  zero (1)  protocol (1)  tcp_length (2)
//! ```
//!
//! IPv6 (40 bytes):
//! ```text
//!   src_addr (16)  dst_addr (16)  upper_layer_length (4 BE)  zero (3)  next_header (1)
//! ```
use crate::net::ip::Ipv4Addr;
use crate::net::ipv6::Ipv6Addr;

#[inline]
fn fold(mut sum: u32) -> u16 {
    while sum >> 16 != 0 { sum = (sum & 0xFFFF) + (sum >> 16); }
    !(sum as u16)
}

pub fn ipv4(src: Ipv4Addr, dst: Ipv4Addr, proto: u8, tcp: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    sum += u16::from_be_bytes([src.0[0], src.0[1]]) as u32;
    sum += u16::from_be_bytes([src.0[2], src.0[3]]) as u32;
    sum += u16::from_be_bytes([dst.0[0], dst.0[1]]) as u32;
    sum += u16::from_be_bytes([dst.0[2], dst.0[3]]) as u32;
    sum += proto as u32;
    sum += tcp.len() as u32;
    let mut i = 0;
    while i + 1 < tcp.len() {
        sum += u16::from_be_bytes([tcp[i], tcp[i + 1]]) as u32;
        i += 2;
    }
    if i < tcp.len() { sum += (tcp[i] as u32) << 8; }
    fold(sum)
}

pub fn ipv6(src: &Ipv6Addr, dst: &Ipv6Addr, next_header: u8, payload: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    for chunk in src.0.chunks(2) { sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32; }
    for chunk in dst.0.chunks(2) { sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32; }
    let len = payload.len() as u32;
    sum += (len >> 16) & 0xFFFF;
    sum += len & 0xFFFF;
    sum += next_header as u32;
    let mut i = 0;
    while i + 1 < payload.len() {
        sum += u16::from_be_bytes([payload[i], payload[i+1]]) as u32;
        i += 2;
    }
    if i < payload.len() { sum += (payload[i] as u32) << 8; }
    fold(sum)
}

#[inline]
pub fn verify_v4(src: Ipv4Addr, dst: Ipv4Addr, proto: u8, tcp: &[u8]) -> bool {
    ipv4(src, dst, proto, tcp) == 0
}

#[inline]
pub fn verify_v6(src: &Ipv6Addr, dst: &Ipv6Addr, next_header: u8, payload: &[u8]) -> bool {
    ipv6(src, dst, next_header, payload) == 0
}
