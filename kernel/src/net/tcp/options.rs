//! TCP option encode/decode (RFC 9293 § 3.2).
//!
//! Stage 7 supports the three options that carry meaningful semantics
//! for a modern endpoint:
//!   * **MSS** (kind 2)        — peer's max-segment-size.
//!   * **Window Scale** (3)    — scale shift applied to advertised window.
//!   * **SACK Permitted** (4)  — peer accepts selective ACKs.
//!
//! Other options are silently skipped (length-byte advanced).
#[derive(Debug, Clone, Copy, Default)]
pub struct ParsedOpts {
    pub mss:           Option<u16>,
    pub window_scale:  Option<u8>,
    pub sack_permitted:bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OptError { Truncated, BadKind, BadLen }

const KIND_END:      u8 = 0;
const KIND_NOP:      u8 = 1;
const KIND_MSS:      u8 = 2;
const KIND_WS:       u8 = 3;
const KIND_SACK_PERM:u8 = 4;

pub fn parse(buf: &[u8]) -> Result<ParsedOpts, OptError> {
    let mut o = ParsedOpts::default();
    let mut p = 0;
    while p < buf.len() {
        let kind = buf[p];
        if kind == KIND_END { break; }
        if kind == KIND_NOP { p += 1; continue; }
        if p + 1 >= buf.len() { return Err(OptError::Truncated); }
        let len = buf[p + 1] as usize;
        if len < 2 || p + len > buf.len() { return Err(OptError::BadLen); }
        match kind {
            KIND_MSS if len == 4 => {
                o.mss = Some(u16::from_be_bytes([buf[p+2], buf[p+3]]));
            }
            KIND_WS if len == 3 => {
                o.window_scale = Some(buf[p + 2].min(14));
            }
            KIND_SACK_PERM if len == 2 => {
                o.sack_permitted = true;
            }
            _ => {}    // unknown — skip
        }
        p += len;
    }
    Ok(o)
}

/// Build a SYN-side option block: MSS + WS + SACK_PERM.
/// Returns the byte count written.
pub fn build_syn(out: &mut [u8], mss: u16, window_scale: u8, sack: bool) -> Option<usize> {
    let need = 4                                   // MSS
             + 3 + 1                               // WS + NOP padding
             + if sack { 2 + 2 } else { 0 };       // SACK_PERM + 2 NOPs to align
    if out.len() < need { return None; }
    let mut p = 0;
    out[p] = KIND_MSS;     out[p+1] = 4;
    out[p+2] = (mss >> 8) as u8; out[p+3] = (mss & 0xFF) as u8;
    p += 4;
    out[p] = KIND_NOP; p += 1;
    out[p] = KIND_WS;  out[p+1] = 3; out[p+2] = window_scale.min(14);
    p += 3;
    if sack {
        out[p] = KIND_NOP; p += 1;
        out[p] = KIND_NOP; p += 1;
        out[p] = KIND_SACK_PERM; out[p+1] = 2;
        p += 2;
    }
    Some(p)
}
