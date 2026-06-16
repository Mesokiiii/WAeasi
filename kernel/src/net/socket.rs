//! Public socket API consumed by `wasi::preview2::sockets`.
//!
//! Stage-8 dual-stack: every TCP/UDP slot stores a `family` so the
//! kernel knows whether to encode v4 or v6 on egress.
use alloc::vec::Vec;

use super::sockaddr::{SocketAddr, SocketAddrV4, SocketAddrV6};
use super::tcp::TcpConnection;
use super::udp::UdpSocket;
use crate::sync::SpinLock;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SocketHandle(pub u32);

#[derive(Debug)]
pub enum SocketError { OutOfHandles, AlreadyBound, InvalidState, Unreachable, BadFamily }

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Family { V4, V6 }

enum Inner {
    Tcp(TcpConnection, Family),
    Udp(UdpSocket, Family),
}

pub struct SocketTable { slots: Vec<Option<Inner>> }

static TABLE: SpinLock<SocketTable> = SpinLock::new(SocketTable { slots: Vec::new() });

fn alloc_slot(t: &mut SocketTable, inner: Inner) -> SocketHandle {
    for (i, s) in t.slots.iter_mut().enumerate() {
        if s.is_none() { *s = Some(inner); return SocketHandle(i as u32); }
    }
    t.slots.push(Some(inner));
    SocketHandle((t.slots.len() - 1) as u32)
}

pub fn create_tcp(v6: bool) -> Result<SocketHandle, SocketError> {
    let fam = if v6 { Family::V6 } else { Family::V4 };
    Ok(alloc_slot(&mut TABLE.lock(), Inner::Tcp(TcpConnection::new(), fam)))
}

pub fn create_udp(v6: bool) -> Result<SocketHandle, SocketError> {
    let fam = if v6 { Family::V6 } else { Family::V4 };
    Ok(alloc_slot(&mut TABLE.lock(), Inner::Udp(UdpSocket::new(), fam)))
}

pub fn tcp_bind(s: SocketHandle, addr: SocketAddr) -> Result<(), SocketError> {
    let mut t = TABLE.lock();
    match t.slots.get_mut(s.0 as usize).and_then(|s| s.as_mut()) {
        Some(Inner::Tcp(c, fam)) => {
            check_family(*fam, &addr)?;
            c.local = Some(addr);
            Ok(())
        }
        _ => Err(SocketError::InvalidState),
    }
}

pub fn tcp_connect(s: SocketHandle, addr: SocketAddr) -> Result<(), SocketError> {
    let mut t = TABLE.lock();
    match t.slots.get_mut(s.0 as usize).and_then(|s| s.as_mut()) {
        Some(Inner::Tcp(c, fam)) => {
            check_family(*fam, &addr)?;
            c.remote = Some(addr);
            Ok(())
        }
        _ => Err(SocketError::InvalidState),
    }
}

pub fn tcp_listen(s: SocketHandle) -> Result<(), SocketError> {
    let mut t = TABLE.lock();
    match t.slots.get_mut(s.0 as usize).and_then(|s| s.as_mut()) {
        Some(Inner::Tcp(c, _)) => {
            let local = c.local.ok_or(SocketError::InvalidState)?;
            c.listen(local);
            Ok(())
        }
        _ => Err(SocketError::InvalidState),
    }
}

fn check_family(want: Family, addr: &SocketAddr) -> Result<(), SocketError> {
    match (want, addr) {
        (Family::V4, SocketAddr::V4(_)) => Ok(()),
        (Family::V6, SocketAddr::V6(_)) => Ok(()),
        _ => Err(SocketError::BadFamily),
    }
}

/// Test helper — keep `SocketAddrV4` / `SocketAddrV6` referenced.
pub const fn _unused_aliases() -> (SocketAddrV4, SocketAddrV6) {
    use crate::net::ip::Ipv4Addr;
    use crate::net::ipv6::Ipv6Addr;
    (
        SocketAddrV4 { ip: Ipv4Addr([0; 4]), port: 0 },
        SocketAddrV6 { ip: Ipv6Addr::UNSPECIFIED, port: 0, flow_info: 0, scope_id: 0 },
    )
}
