//! Neighbor Cache — RFC 4861 § 5.1.
//!
//! Five-state state machine:
//!
//! ```text
//!   INCOMPLETE ──recv NA──► REACHABLE
//!        │                      │
//!        │                      ▼ (timeout)
//!        │                    STALE ──send pkt──► DELAY ──timeout──► PROBE
//!        │                                                              │
//!        │                                                              ▼
//!        └─────────────────── recv NA ──────────────── REACHABLE  ◄─────┘
//! ```
use core::sync::atomic::{AtomicU64, Ordering};

use crate::net::ethernet::MacAddr;
use crate::net::ipv6::Ipv6Addr;
use crate::sync::SpinLock;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NeighborState { Incomplete, Reachable, Stale, Delay, Probe }

#[derive(Copy, Clone, Debug)]
pub struct NeighborEntry {
    pub addr:      Ipv6Addr,
    pub mac:       MacAddr,
    pub state:     NeighborState,
    pub is_router: bool,
    pub ts:        u64,        // monotonic seq for LRU
}

const CACHE_SIZE: usize = 256;

#[derive(Copy, Clone)]
struct Slot { entry: Option<NeighborEntry> }

pub struct NeighborCache { slots: [Slot; CACHE_SIZE], seq: AtomicU64 }

impl NeighborCache {
    pub const fn new() -> Self {
        Self { slots: [Slot { entry: None }; CACHE_SIZE], seq: AtomicU64::new(1) }
    }

    pub fn lookup(&self, addr: &Ipv6Addr) -> Option<NeighborEntry> {
        for s in &self.slots {
            if let Some(e) = s.entry {
                if e.addr == *addr { return Some(e); }
            }
        }
        None
    }

    /// Insert / refresh.  LRU eviction.
    pub fn insert(&mut self, addr: Ipv6Addr, mac: MacAddr, state: NeighborState, is_router: bool) {
        let now = self.seq.fetch_add(1, Ordering::Relaxed);

        // Refresh existing.
        for s in self.slots.iter_mut() {
            if let Some(e) = &mut s.entry {
                if e.addr == addr {
                    e.mac = mac; e.state = state; e.is_router = is_router; e.ts = now;
                    return;
                }
            }
        }
        // Empty slot first.
        for s in self.slots.iter_mut() {
            if s.entry.is_none() {
                s.entry = Some(NeighborEntry { addr, mac, state, is_router, ts: now });
                return;
            }
        }
        // Evict LRU.
        let lru = self.slots.iter_mut().min_by_key(|s| s.entry.map(|e| e.ts).unwrap_or(u64::MAX)).unwrap();
        lru.entry = Some(NeighborEntry { addr, mac, state, is_router, ts: now });
    }

    pub fn invalidate(&mut self, addr: &Ipv6Addr) {
        for s in self.slots.iter_mut() {
            if matches!(s.entry, Some(e) if e.addr == *addr) { s.entry = None; }
        }
    }

    pub fn transition(&mut self, addr: &Ipv6Addr, new_state: NeighborState) {
        for s in self.slots.iter_mut() {
            if let Some(e) = &mut s.entry {
                if e.addr == *addr { e.state = new_state; return; }
            }
        }
    }
}

static CACHE: SpinLock<NeighborCache> = SpinLock::new(NeighborCache::new());

/// Public accessor — returns a guard the caller can call methods on.
pub fn neighbor_cache() -> impl core::ops::DerefMut<Target = NeighborCache> + 'static {
    CACHE.lock()
}
