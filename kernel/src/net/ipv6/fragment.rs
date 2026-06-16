//! IPv6 fragmentation reassembly (RFC 8200 § 4.5).
//!
//! Fragment header (8 bytes):
//! ```text
//!   Next Header (1) | Reserved (1) | Fragment Offset (13) | Res (2) | M (1) | Identification (4)
//! ```
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::sync::SpinLock;

use super::addr::Ipv6Addr;

#[derive(Debug, Clone, Copy)]
pub struct FragmentHeader {
    pub next_header:  u8,
    pub offset_bytes: u16,    // = (raw >> 3) * 8
    pub more:         bool,   // M flag
    pub identification: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FragError { Short, OverlapOrTooLarge, BadFlags }

pub fn parse(buf: &[u8]) -> Result<(FragmentHeader, &[u8]), FragError> {
    if buf.len() < 8 { return Err(FragError::Short); }
    let next  = buf[0];
    let raw   = u16::from_be_bytes([buf[2], buf[3]]);
    let off   = raw & 0xFFF8;
    let more  = (raw & 1) != 0;
    let ident = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
    Ok((FragmentHeader {
        next_header: next, offset_bytes: off, more, identification: ident,
    }, &buf[8..]))
}

/// Reassembly buffer: one entry per (src, dst, identification) tuple.
#[derive(Default)]
struct Reassembly {
    /// Byte-offset → fragment payload.
    chunks:    BTreeMap<u16, Vec<u8>>,
    next_hdr:  u8,
    total_len: Option<u16>,
    started_tsc: u64,
}

const MAX_DATAGRAM:    usize = 65_535;
const MAX_REASSEMBLIES: usize = 1024;
const TIMEOUT_TSC: u64 = 60 * 3_000_000_000;  // 60 s @ 3 GHz heuristic

#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct Key { src: Ipv6Addr, dst: Ipv6Addr, ident: u32 }

static TABLE: SpinLock<BTreeMap<Key, Reassembly>> = SpinLock::new(BTreeMap::new());

/// Feed a single fragment.  When the datagram is complete, returns the
/// reassembled `(next_header, payload)`.  `now_tsc` is supplied by the
/// caller so reassembly can age out stale state.
pub fn ingest(
    src: Ipv6Addr, dst: Ipv6Addr, fh: FragmentHeader,
    payload: &[u8], now_tsc: u64,
) -> Option<(u8, Vec<u8>)> {
    let key = Key { src, dst, ident: fh.identification };
    let mut tbl = TABLE.lock();

    // Sweep timed-out reassemblies (RFC 8200 § 4.5: 60 s).
    tbl.retain(|_, v| now_tsc.wrapping_sub(v.started_tsc) < TIMEOUT_TSC);
    if tbl.len() >= MAX_REASSEMBLIES { return None; }

    let r = tbl.entry(key).or_insert_with(|| Reassembly { started_tsc: now_tsc, ..Default::default() });
    if fh.offset_bytes == 0 { r.next_hdr = fh.next_header; }
    if !fh.more { r.total_len = Some(fh.offset_bytes + payload.len() as u16); }
    r.chunks.insert(fh.offset_bytes, payload.to_vec());

    let tot = r.total_len?;
    if tot as usize > MAX_DATAGRAM { tbl.remove(&Key { src, dst, ident: fh.identification }); return None; }

    let mut have = 0usize;
    for (off, c) in &r.chunks {
        if *off as usize != have { return None; }
        have += c.len();
    }
    if have == tot as usize {
        let r = tbl.remove(&Key { src, dst, ident: fh.identification }).unwrap();
        let mut full = Vec::with_capacity(have);
        for (_, c) in r.chunks { full.extend_from_slice(&c); }
        return Some((r.next_hdr, full));
    }
    None
}
