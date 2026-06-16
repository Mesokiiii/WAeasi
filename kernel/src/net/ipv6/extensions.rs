//! IPv6 extension headers — Hop-by-Hop, Routing, Destination Options,
//! Fragment.
//!
//! Each shares the layout `[next_header u8 | hdr_ext_len u8 | data...]`
//! except the **Fragment** header which is fixed-size (8 bytes).
//!
//! Stage-8 implements:
//!   * `parse_options(buf)` — TLV walker for Hop-by-Hop / Dest Options
//!     (the two share the format).
//!   * `parse_routing(buf)` — Type-0 segments-left tracker (deprecated
//!     by RFC 5095 — we just skip).
//!   * Fragment → `fragment::FragmentHeader`.
#[derive(Copy, Clone, Debug)]
pub struct OptionTlv<'a> {
    pub kind: u8,
    pub data: &'a [u8],
}

#[derive(Debug, PartialEq, Eq)]
pub enum OptError { Short, BadType }

/// Walk a Hop-by-Hop / Destination-Options body (after the 2-byte
/// `next_header / hdr_ext_len` prefix is already stripped).  The first
/// byte of `buf` is the first option's type.
pub fn parse_options(buf: &[u8]) -> Result<alloc::vec::Vec<OptionTlv<'_>>, OptError> {
    let mut out = alloc::vec::Vec::new();
    let mut p = 0;
    while p < buf.len() {
        let kind = buf[p];
        if kind == 0 { p += 1; continue; }       // Pad1
        if p + 1 >= buf.len() { return Err(OptError::Short); }
        let len = buf[p + 1] as usize;
        if p + 2 + len > buf.len() { return Err(OptError::Short); }
        out.push(OptionTlv { kind, data: &buf[p+2 .. p+2+len] });
        p += 2 + len;
    }
    Ok(out)
}

/// Routing-header common shape (RFC 8200 § 4.4):
///   `next_hdr u8 | hdr_ext_len u8 | routing_type u8 | segments_left u8 | data...`
#[derive(Debug)]
pub struct RoutingHdr<'a> {
    pub routing_type:   u8,
    pub segments_left:  u8,
    pub type_specific:  &'a [u8],
}

pub fn parse_routing(buf: &[u8]) -> Result<RoutingHdr<'_>, OptError> {
    if buf.len() < 2 { return Err(OptError::Short); }
    Ok(RoutingHdr {
        routing_type:  buf[0],
        segments_left: buf[1],
        type_specific: &buf[2..],
    })
}
