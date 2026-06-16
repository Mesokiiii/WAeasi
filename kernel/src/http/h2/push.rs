//! HTTP/2 PUSH_PROMISE frame (RFC 9113 § 6.6).
//!
//! ```text
//!   +-+-+-+-+-+-+-+-+-+-+-------------------------------+
//!   |Pad?|R|         Promised Stream ID  (31)           |
//!   +-+-+-+-+-+-+-+-+-+-+-------------------------------+
//!   |                Header Block Fragment (*)          |
//!   +---------------------------------------------------+
//!   |                Padding (*)                        |
//!   +---------------------------------------------------+
//! ```
//!
//! Stage-7 server-push policy:
//!   * The kernel never auto-pushes; every push is explicitly issued by
//!     a Wasm component (`http-server` / `tls-terminator`).
//!   * `EnablePush` SETTINGS bit is honoured — if the peer disables
//!     push, `build_into` refuses with `PushDisabled`.
use crate::http::h2::frame::FrameFlags;

#[derive(Debug, PartialEq, Eq)]
pub enum PushError { Short, PadOversize, PushDisabled }

pub fn build_into(
    out:                 &mut [u8],
    promised_stream_id:  u32,
    header_block:        &[u8],
    padding:             u8,
    push_enabled:        bool,
) -> Result<(usize, FrameFlags), PushError> {
    if !push_enabled { return Err(PushError::PushDisabled); }
    let pad_len = padding as usize;
    let mut total = 4 + header_block.len() + pad_len + if pad_len > 0 { 1 } else { 0 };
    if total > out.len() { return Err(PushError::Short); }

    let mut p = 0;
    let mut flags = FrameFlags::empty();
    if pad_len > 0 {
        if pad_len >= (1 << 8) { return Err(PushError::PadOversize); }
        out[p] = padding; p += 1;
        flags |= FrameFlags::PADDED;
    }
    let sid = (promised_stream_id & 0x7FFF_FFFF).to_be_bytes();
    out[p..p+4].copy_from_slice(&sid); p += 4;
    out[p..p + header_block.len()].copy_from_slice(header_block);
    p += header_block.len();
    for b in &mut out[p..p + pad_len] { *b = 0; }
    p += pad_len;

    total = p;
    Ok((total, flags))
}
