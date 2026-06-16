//! Async TCP/IP stack for `wasi:sockets`.
//!
//! ```text
//!   ethernet → { ipv4 | ipv6 } → { arp | icmp[v6] | ndp | tcp | udp } → socket
//! ```
//!
//! Dual-stack: `socket::Family` selects v4/v6, `route::lookup_v4|v6`
//! finds next-hop, `tcp::checksum::ipv4|ipv6` covers pseudo-header.
pub mod arp;
pub mod ethernet;
pub mod icmp;
pub mod icmpv6;
pub mod ip;
pub mod ipv6;
pub mod mld;
pub mod ndp;
pub mod pmtu;
pub mod route;
pub mod slaac;
pub mod sockaddr;
pub mod socket;
pub mod tcp;
pub mod udp;

pub use sockaddr::{SocketAddr, SocketAddrV4, SocketAddrV6};
