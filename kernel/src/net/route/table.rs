//! Dual-stack routing table.
use alloc::vec::Vec;
use spin::Once;

use crate::net::ip::Ipv4Addr;
use crate::net::ipv6::Ipv6Addr;
use crate::sync::SpinLock;

use super::lpm::Trie;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Family { V4, V6 }

#[derive(Clone, Debug)]
pub struct Route {
    pub family:        Family,
    pub prefix:        [u8; 16],
    pub prefix_length: u8,
    pub gateway:       [u8; 16],
    pub interface_id:  u32,
    pub metric:        u32,
}

#[derive(Default)]
pub struct RoutingTable { v4: Trie<Route>, v6: Trie<Route> }

impl RoutingTable {
    pub fn new() -> Self { Self::default() }

    pub fn add(&mut self, r: Route) {
        let cloned = r.clone();
        match r.family {
            Family::V4 => self.v4.insert(&r.prefix[..4], r.prefix_length as usize, cloned),
            Family::V6 => self.v6.insert(&r.prefix,      r.prefix_length as usize, cloned),
        }
    }

    pub fn remove(&mut self, family: Family, prefix: [u8; 16], prefix_length: u8) {
        match family {
            Family::V4 => self.v4.remove(&prefix[..4], prefix_length as usize),
            Family::V6 => self.v6.remove(&prefix,      prefix_length as usize),
        }
    }

    pub fn lookup_v4(&self, dst: &Ipv4Addr) -> Option<Route> { self.v4.lookup(&dst.0, 32) }
    pub fn lookup_v6(&self, dst: &Ipv6Addr) -> Option<Route> { self.v6.lookup(&dst.0, 128) }
}

static GLOBAL: Once<SpinLock<RoutingTable>> = Once::new();

fn global() -> &'static SpinLock<RoutingTable> {
    GLOBAL.call_once(|| SpinLock::new(RoutingTable::new()))
}

pub fn add(r: Route)    { global().lock().add(r) }
pub fn remove(family: Family, prefix: [u8; 16], prefix_length: u8) {
    global().lock().remove(family, prefix, prefix_length)
}
pub fn lookup_v4(dst: &Ipv4Addr) -> Option<Route> { global().lock().lookup_v4(dst) }
pub fn lookup_v6(dst: &Ipv6Addr) -> Option<Route> { global().lock().lookup_v6(dst) }

pub fn add_default_v6(gateway: Ipv6Addr, interface_id: u32, metric: u32) {
    add(Route {
        family: Family::V6,
        prefix: [0; 16],
        prefix_length: 0,
        gateway: gateway.0,
        interface_id,
        metric,
    });
}

pub fn snapshot() -> Vec<Route> { Vec::new() }
