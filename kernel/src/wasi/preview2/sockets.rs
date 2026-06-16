//! `wasi:sockets/*` — TCP / UDP host bindings on the kernel `net` stack.
//!
//! Stage-8 dual-stack: the `v6` parameter is **honoured** end-to-end —
//! propagates through `socket::create_tcp` / `tcp_bind` / `tcp_connect`
//! and lands in `TcpConnection::local` / `remote` as the right
//! `SocketAddr` variant.
use crate::net::ip::Ipv4Addr;
use crate::net::ipv6::Ipv6Addr;
use crate::net::sockaddr::{SocketAddr, SocketAddrV4, SocketAddrV6};
use crate::net::socket::{self, SocketHandle};
use crate::wasi::errors::{WasiErr, WasiResult};

pub fn create_tcp_socket(v6: bool) -> WasiResult<SocketHandle> {
    socket::create_tcp(v6).map_err(|_| WasiErr::Inval)
}

pub fn create_udp_socket(v6: bool) -> WasiResult<SocketHandle> {
    socket::create_udp(v6).map_err(|_| WasiErr::Inval)
}

pub fn tcp_bind(s: SocketHandle, ip: [u8; 16], port: u16) -> WasiResult<()> {
    let addr = unpack_addr(ip, port);
    socket::tcp_bind(s, addr).map_err(|_| WasiErr::AddrInUse)
}

pub fn tcp_connect(s: SocketHandle, ip: [u8; 16], port: u16) -> WasiResult<()> {
    let addr = unpack_addr(ip, port);
    socket::tcp_connect(s, addr).map_err(|_| WasiErr::ConnRefused)
}

pub fn tcp_listen(s: SocketHandle) -> WasiResult<()> {
    socket::tcp_listen(s).map_err(|_| WasiErr::Io)
}

/// Detect whether a 16-byte input is an IPv4-mapped v6 address
/// (`::ffff:a.b.c.d`).  Convert to `SocketAddrV4` if so; else V6.
fn unpack_addr(ip: [u8; 16], port: u16) -> SocketAddr {
    if is_v4_mapped(&ip) {
        SocketAddr::V4(SocketAddrV4 {
            ip: Ipv4Addr([ip[12], ip[13], ip[14], ip[15]]),
            port,
        })
    } else if is_pure_v4(&ip) {
        SocketAddr::V4(SocketAddrV4 {
            ip: Ipv4Addr([ip[0], ip[1], ip[2], ip[3]]),
            port,
        })
    } else {
        SocketAddr::V6(SocketAddrV6 {
            ip:        Ipv6Addr(ip),
            port,
            flow_info: 0,
            scope_id:  0,
        })
    }
}

#[inline]
fn is_v4_mapped(ip: &[u8; 16]) -> bool {
    ip[..10].iter().all(|&b| b == 0) && ip[10] == 0xFF && ip[11] == 0xFF
}

/// True if the high 12 bytes are zero — a caller passed a v4 in the
/// first 4 bytes.  Heuristic for components that don't yet emit
/// v4-mapped form.
#[inline]
fn is_pure_v4(ip: &[u8; 16]) -> bool {
    ip[4..].iter().all(|&b| b == 0)
}
