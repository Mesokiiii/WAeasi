//! Neighbor Solicitation (NS, ICMPv6 type 135) — RFC 4861 § 4.3.
//!
//! Layout (after 4-byte ICMPv6 header):
//! ```text
//!   Reserved (4) | Target Address (16) | Options (Source Link-Layer)
//! ```
use crate::net::ethernet::MacAddr;
use crate::net::ipv6::Ipv6Addr;

use super::super::icmpv6::header::checksum;
use super::super::icmpv6::ty;
use super::opt;

pub const HEADER_LEN: usize = 4 + 4 + 16;          // icmpv6 + reserved + target
pub const WITH_SLLA:  usize = HEADER_LEN + 8;      // + source-link-layer option

#[derive(Debug)]
pub struct Ns {
    pub target: Ipv6Addr,
    pub source_link_layer: Option<MacAddr>,
}

pub fn parse(buf: &[u8]) -> Option<Ns> {
    if buf.len() < HEADER_LEN { return None; }
    let mut t = [0u8; 16];
    t.copy_from_slice(&buf[8..24]);
    let target = Ipv6Addr(t);

    let mut source_link_layer = None;
    let mut p = HEADER_LEN;
    while p + 2 <= buf.len() {
        let kind = buf[p];
        let len  = buf[p + 1] as usize * 8;
        if len < 2 || p + len > buf.len() { break; }
        if kind == opt::SOURCE_LINK_LAYER && len == 8 {
            let mut m = [0u8; 6];
            m.copy_from_slice(&buf[p+2 .. p+8]);
            source_link_layer = Some(MacAddr(m));
        }
        p += len;
    }
    Some(Ns { target, source_link_layer })
}

pub fn build_into(
    out: &mut [u8],
    src: &Ipv6Addr, dst: &Ipv6Addr,
    target: &Ipv6Addr,
    source_mac: Option<&MacAddr>,
) -> Option<usize> {
    let total = if source_mac.is_some() { WITH_SLLA } else { HEADER_LEN };
    if out.len() < total { return None; }

    out[0] = ty::NEIGHBOR_SOLICIT;
    out[1] = 0;
    out[2] = 0; out[3] = 0;
    out[4..8].fill(0);                              // reserved
    out[8..24].copy_from_slice(&target.0);

    if let Some(mac) = source_mac {
        out[24] = opt::SOURCE_LINK_LAYER;
        out[25] = 1;                                 // 1 × 8 bytes
        out[26..32].copy_from_slice(&mac.0);
    }

    let cs = checksum(src, dst, &out[..total]);
    out[2] = (cs >> 8) as u8;
    out[3] = (cs & 0xFF) as u8;
    Some(total)
}
