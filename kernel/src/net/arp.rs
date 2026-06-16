//! ARP — Address Resolution Protocol (RFC 826).
//!
//! Maps `Ipv4Addr` ↔ `MacAddr`.  Stage-4 cache is a fixed-size LRU; an
//! explicit `valid` bit replaces the `ts == 0`-means-empty heuristic
//! (which was vulnerable to wraparound when SEQ rolled back to 0).
use core::sync::atomic::{AtomicU64, Ordering};

use super::ethernet::MacAddr;
use super::ip::Ipv4Addr;
use crate::sync::SpinLock;

#[repr(u16)]
#[derive(Copy, Clone, Debug)]
pub enum Op { Request = 1, Reply = 2 }

#[repr(C, packed)]
pub struct ArpPacket {
    pub htype: u16, pub ptype: u16,
    pub hlen:  u8,  pub plen:  u8,
    pub op:    u16,
    pub sha: [u8; 6], pub spa: [u8; 4],
    pub tha: [u8; 6], pub tpa: [u8; 4],
}

const CACHE_SIZE: usize = 64;

#[derive(Copy, Clone, Default)]
struct CacheEntry {
    ip:     [u8; 4],
    mac:    [u8; 6],
    ts:     u64,
    valid:  bool,
}

static CACHE: SpinLock<[CacheEntry; CACHE_SIZE]> = SpinLock::new(
    [CacheEntry { ip: [0; 4], mac: [0; 6], ts: 0, valid: false }; CACHE_SIZE]
);
static SEQ: AtomicU64 = AtomicU64::new(1);

/// Insert or refresh a mapping.
pub fn cache_insert(ip: Ipv4Addr, mac: MacAddr) {
    let now = SEQ.fetch_add(1, Ordering::Relaxed);
    let mut t = CACHE.lock();

    // 1. Existing entry → refresh.
    for e in t.iter_mut() {
        if e.valid && e.ip == ip.0 { e.mac = mac.0; e.ts = now; return; }
    }
    // 2. Free slot?
    for e in t.iter_mut() {
        if !e.valid {
            *e = CacheEntry { ip: ip.0, mac: mac.0, ts: now, valid: true };
            return;
        }
    }
    // 3. Evict LRU among valid entries.
    let lru = t.iter_mut().min_by_key(|e| e.ts).unwrap();
    *lru = CacheEntry { ip: ip.0, mac: mac.0, ts: now, valid: true };
}

pub fn cache_lookup(ip: Ipv4Addr) -> Option<MacAddr> {
    let t = CACHE.lock();
    for e in t.iter() {
        if e.valid && e.ip == ip.0 { return Some(MacAddr(e.mac)); }
    }
    None
}

pub fn cache_invalidate(ip: Ipv4Addr) {
    let mut t = CACHE.lock();
    for e in t.iter_mut() {
        if e.valid && e.ip == ip.0 { *e = CacheEntry::default(); return; }
    }
}

/// Build an ARP request packet.  Caller wraps it in an Ethernet frame.
pub fn build_request(local_mac: MacAddr, local_ip: Ipv4Addr, target_ip: Ipv4Addr)
    -> [u8; core::mem::size_of::<ArpPacket>()]
{
    let pkt = ArpPacket {
        htype: 1u16.to_be(),
        ptype: 0x0800u16.to_be(),
        hlen: 6, plen: 4,
        op: (Op::Request as u16).to_be(),
        sha: local_mac.0,
        spa: local_ip.0,
        tha: [0; 6],
        tpa: target_ip.0,
    };
    unsafe { core::mem::transmute(pkt) }
}
