//! Binary-buddy physical allocator.
//!
//! Replaces the stage-1 bump+free-list with O(log N) split/coalesce.
//! Fragmentation is bounded; freeing always merges with the buddy when
//! possible, so memory churn from short-lived Wasm instances stays
//! healthy.
//!
//! Sub-modules split the responsibilities:
//!   * `tree`  — bitmap-backed buddy tree (the core algorithm).
//!   * `slab`  — fixed-size slab caches built on top (for sched/wasm).
//!
//! The buddy itself is wrapped in an IRQ-safe SpinLock; per-CPU magazines
//! sit in front to avoid contention on the hot path (stage 3 work).
pub mod magazine;
pub mod slab;
pub mod tree;

use crate::sync::SpinLock;
use crate::memory::address::PhysAddr;

use tree::BuddyTree;

/// Smallest manageable block — one 4 KiB frame.
pub const MIN_ORDER: u8 = 12;
/// Largest manageable block — 4 MiB (good for huge pages later).
pub const MAX_ORDER: u8 = 22;

pub struct Buddy {
    tree:    BuddyTree,
    base:    PhysAddr,
}

impl Buddy {
    pub fn new() -> Self {
        Self { tree: BuddyTree::empty(), base: PhysAddr::new(0) }
    }

    pub fn init(&mut self, base: PhysAddr, len: usize) {
        self.base = base;
        self.tree.init(base.as_usize(), len, MIN_ORDER, MAX_ORDER);
    }

    /// Allocate `2^order` bytes.  Returns the physical base on success.
    pub fn alloc(&mut self, order: u8) -> Option<PhysAddr> {
        self.tree.alloc(order).map(PhysAddr::new)
    }

    pub fn free(&mut self, p: PhysAddr, order: u8) {
        self.tree.free(p.as_usize(), order);
    }

    /// Telemetry — total bytes still free.
    pub fn free_bytes(&self) -> usize { self.tree.free_bytes() }
}

static GLOBAL: SpinLock<Buddy> = SpinLock::new(Buddy {
    tree: BuddyTree::empty(),
    base: PhysAddr::new(0),
});

pub fn init(base: PhysAddr, len: usize) {
    GLOBAL.lock().init(base, len);
    log::info!("[buddy] {} MiB managed @ {:?}", len / (1024 * 1024), base);
}

pub fn alloc(order: u8) -> Option<PhysAddr> { GLOBAL.lock().alloc(order) }
pub fn free(p: PhysAddr, order: u8) { GLOBAL.lock().free(p, order) }
pub fn free_bytes() -> usize { GLOBAL.lock().free_bytes() }
