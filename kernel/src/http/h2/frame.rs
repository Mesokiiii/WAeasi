//! HTTP/2 frame header.
//!
//! ```text
//!   length (24)  type (8)  flags (8)  reserved (1) | stream_id (31)
//! ```
use bitflags::bitflags;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameType {
    Data         = 0x0,
    Headers      = 0x1,
    Priority     = 0x2,
    RstStream    = 0x3,
    Settings     = 0x4,
    PushPromise  = 0x5,
    Ping         = 0x6,
    Goaway       = 0x7,
    WindowUpdate = 0x8,
    Continuation = 0x9,
    Other        = 0xFF,
}

impl FrameType {
    pub fn from_u8(b: u8) -> Self {
        match b {
            0x0 => Self::Data,         0x1 => Self::Headers,
            0x2 => Self::Priority,     0x3 => Self::RstStream,
            0x4 => Self::Settings,     0x5 => Self::PushPromise,
            0x6 => Self::Ping,         0x7 => Self::Goaway,
            0x8 => Self::WindowUpdate, 0x9 => Self::Continuation,
            _   => Self::Other,
        }
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FrameFlags: u8 {
        const END_STREAM   = 0x01;
        const ACK          = 0x01;     // overlaps END_STREAM by frame type
        const END_HEADERS  = 0x04;
        const PADDED       = 0x08;
        const PRIORITY     = 0x20;
    }
}

#[derive(Debug, Copy, Clone)]
pub struct FrameHeader {
    pub length:    u32,
    pub kind:      FrameType,
    pub flags:     FrameFlags,
    pub stream_id: u32,
}

pub const HEADER_LEN: usize = 9;

#[derive(Debug, PartialEq, Eq)]
pub enum FrameError { Short, Oversize }

pub fn parse_header(buf: &[u8]) -> Result<FrameHeader, FrameError> {
    if buf.len() < HEADER_LEN { return Err(FrameError::Short); }
    let length = ((buf[0] as u32) << 16) | ((buf[1] as u32) << 8) | (buf[2] as u32);
    if length > (1 << 24) - 1 { return Err(FrameError::Oversize); }
    let kind = FrameType::from_u8(buf[3]);
    let flags = FrameFlags::from_bits_truncate(buf[4]);
    let stream_id = u32::from_be_bytes([buf[5], buf[6], buf[7], buf[8]]) & 0x7FFF_FFFF;
    Ok(FrameHeader { length, kind, flags, stream_id })
}

/// Write a frame header into `out`.  Returns the byte count.
pub fn write_header(out: &mut [u8], h: FrameHeader) -> Result<usize, FrameError> {
    if out.len() < HEADER_LEN { return Err(FrameError::Short); }
    out[0] = (h.length >> 16) as u8;
    out[1] = (h.length >>  8) as u8;
    out[2] = (h.length & 0xFF) as u8;
    out[3] = h.kind as u8;
    out[4] = h.flags.bits();
    let sid = (h.stream_id & 0x7FFF_FFFF).to_be_bytes();
    out[5..9].copy_from_slice(&sid);
    Ok(HEADER_LEN)
}
