//! Unified `SocketAddr` — abstracts over IPv4 and IPv6.
//!
//! Replaces the historical `[u8; 16]` blob in `tcp::TcpConnection` so
//! the rest of the kernel can pattern-match on family without parsing
//! the address bytes.
use crate::net::ip::Ipv4Addr;
use crate::net::ipv6::Ipv6Addr;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SocketAddrV4 { pub ip: Ipv4Addr, pub port: u16 }

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SocketAddrV6 {
    pub ip:        Ipv6Addr,
    pub port:      u16,
    pub flow_info: u32,
    pub scope_id:  u32,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SocketAddr {
    V4(SocketAddrV4),
    V6(SocketAddrV6),
}

impl SocketAddr {
    #[inline] pub fn is_v6(&self)  -> bool { matches!(self, SocketAddr::V6(_)) }
    #[inline] pub fn is_v4(&self)  -> bool { matches!(self, SocketAddr::V4(_)) }
    #[inline] pub fn port(&self)   -> u16 {
        match self { SocketAddr::V4(a) => a.port, SocketAddr::V6(a) => a.port }
    }

    /// `IN6ADDR_ANY` semantics — listen on every interface.
    pub const ANY_V6: Self = Self::V6(SocketAddrV6 {
        ip:        Ipv6Addr::UNSPECIFIED,
        port:      0,
        flow_info: 0,
        scope_id:  0,
    });

    /// `INADDR_ANY` semantics for v4.
    pub const ANY_V4: Self = Self::V4(SocketAddrV4 {
        ip:   Ipv4Addr([0; 4]),
        port: 0,
    });

    /// IPv4-mapped IPv6 address (RFC 4291 § 2.5.5.2): a v4 endpoint
    /// reachable through a v6 dual-stack socket.
    pub fn v4_mapped_v6(addr: Ipv4Addr, port: u16) -> Self {
        let mut bytes = [0u8; 16];
        bytes[10] = 0xFF; bytes[11] = 0xFF;
        bytes[12..16].copy_from_slice(&addr.0);
        SocketAddr::V6(SocketAddrV6 {
            ip: Ipv6Addr(bytes), port, flow_info: 0, scope_id: 0,
        })
    }
}
