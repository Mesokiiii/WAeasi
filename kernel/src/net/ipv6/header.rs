//! IPv6 fixed header (RFC 8200 § 3).
//!
//! ```text
//!   0                   1                   2                   3
//!   0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//!  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//!  |Version| Traffic Class |           Flow Label                  |
//!  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//!  |         Payload Length        |  Next Header  |   Hop Limit   |
//!  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//!  |                         Source Address                        |
//!  +                                                               +
//!  |                                                               |
//!  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//!  |                       Destination Address                     |
//!  +                                                               +
//!  |                                                               |
//!  +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```
//!
//! Total fixed-header size = 40 bytes.
use super::addr::Ipv6Addr;

pub const HEADER_LEN:    usize = 40;
pub const DEFAULT_HOP:   u8    = 64;
pub const VERSION_NIBBLE:u8    = 6;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct NextHeader(pub u8);

#[derive(Debug, Clone)]
pub struct Ipv6Header {
    pub traffic_class: u8,
    pub flow_label:    u32,    // 20 bits
    pub payload_len:   u16,    // bytes following this fixed header
    pub next_header:   NextHeader,
    pub hop_limit:     u8,
    pub src:           Ipv6Addr,
    pub dst:           Ipv6Addr,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError { Short, BadVersion }

pub fn parse(buf: &[u8]) -> Result<(Ipv6Header, &[u8]), ParseError> {
    if buf.len() < HEADER_LEN { return Err(ParseError::Short); }
    let v = (buf[0] >> 4) & 0x0F;
    if v != VERSION_NIBBLE { return Err(ParseError::BadVersion); }

    let traffic_class = ((buf[0] & 0x0F) << 4) | (buf[1] >> 4);
    let flow_label    = (((buf[1] & 0x0F) as u32) << 16)
                      | ((buf[2] as u32) << 8)
                      |  (buf[3] as u32);
    let payload_len   = u16::from_be_bytes([buf[4], buf[5]]);
    let next_header   = NextHeader(buf[6]);
    let hop_limit     = buf[7];

    let mut src_b = [0u8; 16]; src_b.copy_from_slice(&buf[8..24]);
    let mut dst_b = [0u8; 16]; dst_b.copy_from_slice(&buf[24..40]);

    let total = HEADER_LEN + payload_len as usize;
    if buf.len() < total { return Err(ParseError::Short); }

    Ok((
        Ipv6Header {
            traffic_class, flow_label, payload_len, next_header, hop_limit,
            src: Ipv6Addr(src_b), dst: Ipv6Addr(dst_b),
        },
        &buf[HEADER_LEN..total],
    ))
}

/// Build a fixed header into `out[0..40]`.  Returns the byte count.
pub fn build_into(out: &mut [u8], h: &Ipv6Header) -> Result<usize, ParseError> {
    if out.len() < HEADER_LEN { return Err(ParseError::Short); }
    out[0] = (VERSION_NIBBLE << 4) | (h.traffic_class >> 4);
    out[1] = ((h.traffic_class & 0x0F) << 4) | ((h.flow_label >> 16) & 0x0F) as u8;
    out[2] = (h.flow_label >> 8) as u8;
    out[3] =  h.flow_label as u8;
    out[4..6].copy_from_slice(&h.payload_len.to_be_bytes());
    out[6] = h.next_header.0;
    out[7] = h.hop_limit;
    out[8..24] .copy_from_slice(&h.src.0);
    out[24..40].copy_from_slice(&h.dst.0);
    Ok(HEADER_LEN)
}

/// Walk the Next-Header chain past extension headers, returning the
/// upper-layer protocol number and the byte offset of its data.
///
/// Stage 8 supports: Hop-by-Hop (0), Routing (43), Destination
/// Options (60), Fragment (44).  AH/ESP are surfaced unchanged so a
/// downstream IPSec layer can pick them up.
pub fn skip_extensions(mut next: u8, mut buf: &[u8]) -> (u8, usize) {
    let mut consumed = 0;
    loop {
        let is_ext = matches!(next, 0 | 43 | 60 | 44);
        if !is_ext || buf.len() < 2 { break; }
        let new_next = buf[0];
        let len_bytes = if next == 44 { 8 } else { (buf[1] as usize + 1) * 8 };
        if buf.len() < len_bytes { break; }
        consumed += len_bytes;
        buf = &buf[len_bytes..];
        next = new_next;
    }
    (next, consumed)
}
