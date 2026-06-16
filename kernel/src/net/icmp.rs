//! ICMP — Internet Control Message Protocol (RFC 792).
//!
//! Stage 4 implements **echo reply** (ping) — the only ICMP type a
//! cloud-native OS reasonably needs.
//!
//! DoS hardening:
//!   * `MAX_ECHO_PAYLOAD = 1500` — refuse echoes larger than a normal
//!     Ethernet MTU.  Without this, a single malicious peer can push us
//!     into 64 KiB allocations per packet.
//!   * `handle()` writes the reply **into a caller-provided buffer**
//!     instead of `Vec::with_capacity(packet.len())` — zero allocation
//!     on the hot path.
use super::ip::Ipv4Addr;

#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum IcmpType { EchoReply = 0, EchoRequest = 8 }

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct IcmpHeader {
    pub kind:     u8,
    pub code:     u8,
    pub checksum: u16,
    pub id:       u16,
    pub seq:      u16,
}

pub const HEADER_LEN:        usize = core::mem::size_of::<IcmpHeader>();
pub const MAX_ECHO_PAYLOAD:  usize = 1500;

#[derive(Debug, PartialEq, Eq)]
pub enum IcmpError { Oversize, ShortBuffer, NotEcho }

/// One's-complement checksum.
pub fn checksum(bytes: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < bytes.len() {
        sum += u16::from_be_bytes([bytes[i], bytes[i + 1]]) as u32;
        i += 2;
    }
    if i < bytes.len() { sum += (bytes[i] as u32) << 8; }
    while sum >> 16 != 0 { sum = (sum & 0xFFFF) + (sum >> 16); }
    !(sum as u16)
}

/// Try to handle an inbound ICMP packet.  On `EchoRequest`, writes the
/// reply into `out` and returns the number of bytes written.
///
/// Returns `Err(NotEcho)` for any non-echo type (caller can ignore).
pub fn handle(packet: &[u8], _src: Ipv4Addr, out: &mut [u8]) -> Result<usize, IcmpError> {
    if packet.len() < HEADER_LEN { return Err(IcmpError::ShortBuffer); }
    if packet.len() > HEADER_LEN + MAX_ECHO_PAYLOAD { return Err(IcmpError::Oversize); }
    if out.len() < packet.len() { return Err(IcmpError::ShortBuffer); }

    let kind = packet[0];
    if kind != IcmpType::EchoRequest as u8 { return Err(IcmpError::NotEcho); }

    out[..packet.len()].copy_from_slice(packet);
    out[0] = IcmpType::EchoReply as u8;
    out[2] = 0; out[3] = 0;
    let cs = checksum(&out[..packet.len()]);
    out[2] = (cs >> 8) as u8;
    out[3] = (cs & 0xFF) as u8;
    Ok(packet.len())
}
