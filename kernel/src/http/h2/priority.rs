//! HTTP/2 PRIORITY frame (RFC 9113 § 6.3).
//!
//! ```text
//!   +-+-------------+------------------------------+
//!   |E|   31-bit Stream Dependency                 |
//!   +-+-------------+------------------------------+
//!   |  Weight (1 byte; actual weight = value + 1)  |
//!   +----------------------------------------------+
//! ```
//!
//! Stage 7 emits/parses but does not yet inject priorities into the
//! scheduler — that lives in the executor's stream queue and lands in
//! Stage 8 alongside connection-level flow control.
#[derive(Debug, Copy, Clone)]
pub struct Priority {
    pub exclusive:  bool,
    pub depends_on: u32,
    pub weight:     u8,         // 1..=256 (encoded as 0..=255)
}

#[derive(Debug, PartialEq, Eq)]
pub enum PriorityError { Short }

pub const FRAME_LEN: usize = 5;

pub fn parse(buf: &[u8]) -> Result<Priority, PriorityError> {
    if buf.len() < FRAME_LEN { return Err(PriorityError::Short); }
    let raw = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
    let exclusive = raw & 0x8000_0000 != 0;
    let depends_on = raw & 0x7FFF_FFFF;
    let weight = buf[4].wrapping_add(1);
    Ok(Priority { exclusive, depends_on, weight })
}

pub fn write(out: &mut [u8], p: Priority) -> Result<usize, PriorityError> {
    if out.len() < FRAME_LEN { return Err(PriorityError::Short); }
    let mut raw = p.depends_on & 0x7FFF_FFFF;
    if p.exclusive { raw |= 0x8000_0000; }
    out[..4].copy_from_slice(&raw.to_be_bytes());
    out[4] = p.weight.saturating_sub(1);
    Ok(FRAME_LEN)
}
