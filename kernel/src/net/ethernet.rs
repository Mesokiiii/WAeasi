//! Layer 2 — Ethernet frame encode/decode.
use core::convert::TryInto;

pub const MAC_LEN: usize = 6;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MacAddr(pub [u8; MAC_LEN]);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum EtherType { Ipv4 = 0x0800, Arp = 0x0806, Ipv6 = 0x86DD, Unknown = 0xFFFF }

impl EtherType {
    pub fn from_be(v: u16) -> Self {
        match v {
            0x0800 => Self::Ipv4,
            0x0806 => Self::Arp,
            0x86DD => Self::Ipv6,
            _      => Self::Unknown,
        }
    }
}

#[derive(Debug)]
pub struct EthernetFrame<'a> {
    pub dst: MacAddr,
    pub src: MacAddr,
    pub kind: EtherType,
    pub payload: &'a [u8],
}

pub fn parse(buf: &[u8]) -> Option<EthernetFrame<'_>> {
    if buf.len() < 14 { return None; }
    let dst  = MacAddr(buf[0..6].try_into().ok()?);
    let src  = MacAddr(buf[6..12].try_into().ok()?);
    let kind = EtherType::from_be(u16::from_be_bytes([buf[12], buf[13]]));
    Some(EthernetFrame { dst, src, kind, payload: &buf[14..] })
}

pub fn encode(out: &mut [u8], frame: &EthernetFrame<'_>) -> Option<usize> {
    if out.len() < 14 + frame.payload.len() { return None; }
    out[0..6].copy_from_slice(&frame.dst.0);
    out[6..12].copy_from_slice(&frame.src.0);
    out[12..14].copy_from_slice(&(frame.kind as u16).to_be_bytes());
    out[14..14 + frame.payload.len()].copy_from_slice(frame.payload);
    Some(14 + frame.payload.len())
}
