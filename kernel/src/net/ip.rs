//! Layer 3 — IPv4/IPv6 address types and minimal header parsing.
use core::fmt;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Ipv4Addr(pub [u8; 4]);
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Ipv6Addr(pub [u8; 16]);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum IpAddr { V4(Ipv4Addr), V6(Ipv6Addr) }

impl fmt::Debug for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [a,b,c,d] = self.0;
        write!(f, "{}.{}.{}.{}", a, b, c, d)
    }
}

impl fmt::Debug for Ipv6Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, chunk) in self.0.chunks(2).enumerate() {
            if i > 0 { f.write_str(":")?; }
            write!(f, "{:02x}{:02x}", chunk[0], chunk[1])?;
        }
        Ok(())
    }
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum L4Proto { Tcp = 6, Udp = 17, Icmp = 1, Other = 0xFF }

/// Minimal IPv4 header parser — used by sockets layer.
pub struct Ipv4Header {
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
    pub proto: L4Proto,
    pub payload_off: usize,
    pub total_len: u16,
}

pub fn parse_v4(buf: &[u8]) -> Option<Ipv4Header> {
    if buf.len() < 20 { return None; }
    let ihl = (buf[0] & 0x0F) as usize * 4;
    if buf.len() < ihl { return None; }
    let proto = match buf[9] { 6 => L4Proto::Tcp, 17 => L4Proto::Udp, 1 => L4Proto::Icmp, _ => L4Proto::Other };
    let src = Ipv4Addr([buf[12], buf[13], buf[14], buf[15]]);
    let dst = Ipv4Addr([buf[16], buf[17], buf[18], buf[19]]);
    let total_len = u16::from_be_bytes([buf[2], buf[3]]);
    Some(Ipv4Header { src, dst, proto, payload_off: ihl, total_len })
}
