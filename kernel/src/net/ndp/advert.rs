//! Neighbor Advertisement (NA, ICMPv6 type 136) — RFC 4861 § 4.4.
//!
//! Layout (after 4-byte ICMPv6 header):
//! ```text
//!   R|S|O|Reserved (29 bits) | Target Address (16) | Options (Target Link-Layer)
//! ```
//!   R: Router flag        (sender is a router)
//!   S: Solicited flag     (response to specific NS)
//!   O: Override flag      (overrides existing cache entry)
use crate::net::ethernet::MacAddr;
use crate::net::ipv6::Ipv6Addr;

use super::super::icmpv6::header::checksum;
use super::super::icmpv6::ty;
use super::opt;

pub const HEADER_LEN: usize = 4 + 4 + 16;
pub const WITH_TLLA:  usize = HEADER_LEN + 8;

#[derive(Debug)]
pub struct Na {
    pub router:    bool,
    pub solicited: bool,
    pub override_: bool,
    pub target:    Ipv6Addr,
    pub target_link_layer: Option<MacAddr>,
}

pub fn parse(buf: &[u8]) -> Option<Na> {
    if buf.len() < HEADER_LEN { return None; }
    let flags = buf[4];
    let router    = flags & 0x80 != 0;
    let solicited = flags & 0x40 != 0;
    let override_ = flags & 0x20 != 0;

    let mut t = [0u8; 16];
    t.copy_from_slice(&buf[8..24]);

    let mut target_link_layer = None;
    let mut p = HEADER_LEN;
    while p + 2 <= buf.len() {
        let kind = buf[p];
        let len  = buf[p + 1] as usize * 8;
        if len < 2 || p + len > buf.len() { break; }
        if kind == opt::TARGET_LINK_LAYER && len == 8 {
            let mut m = [0u8; 6];
            m.copy_from_slice(&buf[p+2 .. p+8]);
            target_link_layer = Some(MacAddr(m));
        }
        p += len;
    }
    Some(Na {
        router, solicited, override_,
        target: Ipv6Addr(t), target_link_layer,
    })
}

pub fn build_into(
    out: &mut [u8],
    src: &Ipv6Addr, dst: &Ipv6Addr,
    target: &Ipv6Addr,
    flags: NaFlags,
    target_mac: Option<&MacAddr>,
) -> Option<usize> {
    let total = if target_mac.is_some() { WITH_TLLA } else { HEADER_LEN };
    if out.len() < total { return None; }

    out[0] = ty::NEIGHBOR_ADVERT;
    out[1] = 0;
    out[2] = 0; out[3] = 0;
    out[4] = 0;
    if flags.router    { out[4] |= 0x80; }
    if flags.solicited { out[4] |= 0x40; }
    if flags.override_ { out[4] |= 0x20; }
    out[5..8].fill(0);
    out[8..24].copy_from_slice(&target.0);

    if let Some(mac) = target_mac {
        out[24] = opt::TARGET_LINK_LAYER;
        out[25] = 1;
        out[26..32].copy_from_slice(&mac.0);
    }

    let cs = checksum(src, dst, &out[..total]);
    out[2] = (cs >> 8) as u8;
    out[3] = (cs & 0xFF) as u8;
    Some(total)
}

#[derive(Copy, Clone, Debug, Default)]
pub struct NaFlags {
    pub router:    bool,
    pub solicited: bool,
    pub override_: bool,
}
