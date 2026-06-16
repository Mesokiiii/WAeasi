//! MLD — Multicast Listener Discovery (RFC 3810, MLDv2).
//!
//! Required to participate in any IPv6 multicast group, including the
//! link-local `ff02::1` (all-nodes) and the per-host solicited-node
//! multicasts NDP relies on.
//!
//! Stage-8 minimum: `Listener Report` (type 143 for v2, 131 for v1) and
//! `Listener Done` (type 132) — what a stub host needs to announce
//! itself on the link.
use alloc::vec::Vec;

use crate::net::ipv6::Ipv6Addr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordType {
    /// MODE_IS_INCLUDE
    IsInclude        = 1,
    /// MODE_IS_EXCLUDE
    IsExclude        = 2,
    /// CHANGE_TO_INCLUDE
    ToInclude        = 3,
    /// CHANGE_TO_EXCLUDE
    ToExclude        = 4,
    /// ALLOW_NEW_SOURCES
    AllowSources     = 5,
    /// BLOCK_OLD_SOURCES
    BlockSources     = 6,
}

#[derive(Debug, Clone)]
pub struct GroupRecord {
    pub kind:    RecordType,
    pub group:   Ipv6Addr,
    pub sources: Vec<Ipv6Addr>,
}

/// Build an MLDv2 Listener Report (ICMPv6 type 143).
///
/// Layout:
/// ```text
///   Type=143 | Code=0 | Checksum
///   Reserved (2)  | Number of Records (2)
///   Records...
///       Record Type (1) | Aux Len (1) | Number of Sources (2)
///       Multicast Address (16)
///       Sources... (16 each)
///       Auxiliary Data...
/// ```
pub fn build_report_into(
    out: &mut [u8],
    src: &Ipv6Addr, dst: &Ipv6Addr,
    records: &[GroupRecord],
) -> Option<usize> {
    use crate::net::icmpv6::header::checksum;
    use crate::net::icmpv6::ty;

    let mut size = 8;
    for r in records { size += 4 + 16 + r.sources.len() * 16; }
    if out.len() < size { return None; }

    out[0] = ty::MLD_REPORT_V2;
    out[1] = 0;
    out[2] = 0; out[3] = 0;
    out[4] = 0; out[5] = 0;
    out[6..8].copy_from_slice(&(records.len() as u16).to_be_bytes());

    let mut p = 8;
    for r in records {
        out[p]     = r.kind as u8;
        out[p + 1] = 0;                                  // aux data length
        out[p+2..p+4].copy_from_slice(&(r.sources.len() as u16).to_be_bytes());
        out[p+4..p+20].copy_from_slice(&r.group.0);
        p += 20;
        for s in &r.sources {
            out[p..p+16].copy_from_slice(&s.0);
            p += 16;
        }
    }
    let cs = checksum(src, dst, &out[..p]);
    out[2] = (cs >> 8) as u8;
    out[3] = (cs & 0xFF) as u8;
    Some(p)
}

/// Compatibility shim — exposes V2 type number through the icmpv6
/// surface so other modules don't pull in this entire file.
pub const TYPE_MLDV2_REPORT: u8 = 143;
