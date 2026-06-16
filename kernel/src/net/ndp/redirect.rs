//! Redirect message (ICMPv6 type 137) — RFC 4861 § 4.5.
//!
//! Sent by a router telling a host that a better first-hop exists for
//! a particular destination.  Updates the host's destination cache
//! (next-hop selection), not the routing table.
//!
//! ```text
//!   Reserved (4) | Target Address (16) | Destination Address (16) | Options
//! ```
use crate::net::ipv6::Ipv6Addr;

#[derive(Debug)]
pub struct Redirect {
    pub target:      Ipv6Addr,
    pub destination: Ipv6Addr,
}

pub fn parse(buf: &[u8]) -> Option<Redirect> {
    if buf.len() < 4 + 4 + 16 + 16 { return None; }
    let mut t = [0u8; 16]; t.copy_from_slice(&buf[8..24]);
    let mut d = [0u8; 16]; d.copy_from_slice(&buf[24..40]);
    Some(Redirect { target: Ipv6Addr(t), destination: Ipv6Addr(d) })
}
