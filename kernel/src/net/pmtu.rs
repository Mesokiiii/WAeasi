//! Path MTU Discovery for IPv6 (RFC 8201).
//!
//! IPv6 routers **never** fragment in transit; the source must learn the
//! path MTU through `ICMPv6 Packet Too Big` messages and reduce its
//! emission size accordingly.  We cache one MTU per (src, dst) pair.
use alloc::collections::BTreeMap;

use crate::net::ipv6::Ipv6Addr;
use crate::sync::SpinLock;

pub const IPV6_MIN_MTU:  u32 = 1280;
pub const IPV6_DEFAULT_MTU: u32 = 1500;

#[derive(Eq, Ord, PartialEq, PartialOrd, Clone, Copy)]
struct Key { src: Ipv6Addr, dst: Ipv6Addr }

#[derive(Copy, Clone, Debug)]
pub struct Entry {
    pub mtu:        u32,
    pub last_tsc:   u64,
}

const AGEOUT_TSC: u64 = 600 * 3_000_000_000;          // 10 min @ 3 GHz heuristic

static TABLE: SpinLock<BTreeMap<Key, Entry>> = SpinLock::new(BTreeMap::new());

/// Look up the cached MTU.  Falls back to `IPV6_DEFAULT_MTU` if no entry.
pub fn lookup(src: Ipv6Addr, dst: Ipv6Addr) -> u32 {
    TABLE.lock().get(&Key { src, dst }).map(|e| e.mtu).unwrap_or(IPV6_DEFAULT_MTU)
}

/// Process an inbound `Packet Too Big` (Type 2).  `mtu_advertised` must
/// be at least `IPV6_MIN_MTU`; smaller values are clamped per RFC 8201.
pub fn on_packet_too_big(src: Ipv6Addr, dst: Ipv6Addr, mtu_advertised: u32, now_tsc: u64) {
    let mtu = mtu_advertised.max(IPV6_MIN_MTU);
    TABLE.lock().insert(Key { src, dst }, Entry { mtu, last_tsc: now_tsc });
}

/// Periodic GC — drop stale entries.  Stage 9 wires this to the
/// scheduler reactor as a 60-second tick.
pub fn gc(now_tsc: u64) {
    let mut t = TABLE.lock();
    t.retain(|_, v| now_tsc.wrapping_sub(v.last_tsc) < AGEOUT_TSC);
}

/// Manually clear the entry — used after explicit reachability change.
pub fn invalidate(src: Ipv6Addr, dst: Ipv6Addr) {
    TABLE.lock().remove(&Key { src, dst });
}
