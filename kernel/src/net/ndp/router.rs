//! Router Solicitation (RS, type 133) and Router Advertisement (RA, type 134).
//!
//! RA carries:
//!   * `Cur Hop Limit` — value to use in outgoing packets.
//!   * `M, O` flags — Managed/Other config (DHCPv6 hints).
//!   * `Router Lifetime` (default-router lifetime in seconds).
//!   * `Reachable Time`, `Retrans Timer`.
//!   * Options: Source LLA, MTU, **Prefix Information** (used by SLAAC).
use crate::net::ethernet::MacAddr;
use crate::net::ipv6::Ipv6Addr;

use super::super::icmpv6::header::checksum;
use super::super::icmpv6::ty;
use super::opt;

#[derive(Debug)]
pub struct Ra {
    pub cur_hop_limit:  u8,
    pub managed:        bool,
    pub other_config:   bool,
    pub router_lifetime:u16,    // seconds; 0 = not a default router
    pub reachable_time: u32,    // ms
    pub retrans_timer:  u32,    // ms
    pub source_link_layer: Option<MacAddr>,
    pub mtu:            Option<u32>,
    pub prefixes:       alloc::vec::Vec<PrefixInfo>,
}

#[derive(Debug, Clone, Copy)]
pub struct PrefixInfo {
    pub prefix_length:    u8,
    pub on_link:          bool,
    pub autonomous:       bool,
    pub valid_lifetime:   u32,
    pub preferred_lifetime:u32,
    pub prefix:           Ipv6Addr,
}

pub fn parse_ra(buf: &[u8]) -> Option<Ra> {
    // ICMPv6 header (4) + RA fixed (12) = 16
    if buf.len() < 16 { return None; }
    let cur_hop_limit  = buf[4];
    let flags          = buf[5];
    let router_lifetime= u16::from_be_bytes([buf[6], buf[7]]);
    let reachable_time = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);
    let retrans_timer  = u32::from_be_bytes([buf[12], buf[13], buf[14], buf[15]]);

    let mut ra = Ra {
        cur_hop_limit,
        managed:      flags & 0x80 != 0,
        other_config: flags & 0x40 != 0,
        router_lifetime, reachable_time, retrans_timer,
        source_link_layer: None,
        mtu: None,
        prefixes: alloc::vec::Vec::new(),
    };

    let mut p = 16;
    while p + 2 <= buf.len() {
        let kind = buf[p];
        let len  = buf[p + 1] as usize * 8;
        if len < 8 || p + len > buf.len() { break; }
        match kind {
            opt::SOURCE_LINK_LAYER if len == 8 => {
                let mut m = [0u8; 6]; m.copy_from_slice(&buf[p+2..p+8]);
                ra.source_link_layer = Some(MacAddr(m));
            }
            opt::MTU if len == 8 => {
                ra.mtu = Some(u32::from_be_bytes([buf[p+4], buf[p+5], buf[p+6], buf[p+7]]));
            }
            opt::PREFIX_INFORMATION if len == 32 => {
                let mut prefix = [0u8; 16];
                prefix.copy_from_slice(&buf[p+16..p+32]);
                ra.prefixes.push(PrefixInfo {
                    prefix_length:      buf[p+2],
                    on_link:            buf[p+3] & 0x80 != 0,
                    autonomous:         buf[p+3] & 0x40 != 0,
                    valid_lifetime:     u32::from_be_bytes([buf[p+4], buf[p+5], buf[p+6], buf[p+7]]),
                    preferred_lifetime: u32::from_be_bytes([buf[p+8], buf[p+9], buf[p+10], buf[p+11]]),
                    prefix:             Ipv6Addr(prefix),
                });
            }
            _ => {}
        }
        p += len;
    }
    Some(ra)
}

/// Build a Router Solicitation — sent by the host on link to discover routers.
pub fn build_rs_into(
    out: &mut [u8],
    src: &Ipv6Addr, dst: &Ipv6Addr,
    source_mac: Option<&MacAddr>,
) -> Option<usize> {
    let base = 4 + 4;     // icmpv6 + reserved
    let total = if source_mac.is_some() { base + 8 } else { base };
    if out.len() < total { return None; }
    out[0] = ty::ROUTER_SOLICIT;
    out[1] = 0;
    out[2] = 0; out[3] = 0;
    out[4..8].fill(0);
    if let Some(mac) = source_mac {
        out[8] = opt::SOURCE_LINK_LAYER;
        out[9] = 1;
        out[10..16].copy_from_slice(&mac.0);
    }
    let cs = checksum(src, dst, &out[..total]);
    out[2] = (cs >> 8) as u8;
    out[3] = (cs & 0xFF) as u8;
    Some(total)
}
