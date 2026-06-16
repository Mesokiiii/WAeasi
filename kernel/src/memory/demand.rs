//! Demand paging for Wasm linear memories.
//!
//! Hot path is `try_handle(va)`, called from the `#PF` handler:
//!
//!   * **IRQ-safe** — uses `crate::sync::SpinLock`, not `spin::Mutex`.
//!     Page faults arrive in exception context (IRQs disabled by the
//!     interrupt gate), but the **registration** side runs in task
//!     context with IRQs on — without our SpinLock the cross-CPU pair
//!     could deadlock.
//!
//!   * **O(log N) lookup** — regions are stored in a sorted `Vec` keyed
//!     by `base.as_usize()`.  A `partition_point` binary search locates
//!     the candidate region in `log₂ N` comparisons.  At trillions of
//!     PFs/sec with 64 K registered ranges this is the difference between
//!     "feasible" and "the kernel disappears under PF cost".
//!
//! Stage 4 will replace the Vec with an interval B-tree once we routinely
//! see >1 M concurrent regions per node.
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use super::address::{PhysAddr, VirtAddr};
use super::frame_alloc;
use super::page_table;
use super::paging::{phys_to_virt, PageFlags};
use crate::sync::SpinLock;

#[derive(Copy, Clone, Debug)]
pub struct Region {
    pub base: VirtAddr,
    pub end:  VirtAddr,
}

static REGIONS: SpinLock<Vec<Region>> = SpinLock::new(Vec::new());
static FAULTED_IN: AtomicUsize = AtomicUsize::new(0);

/// Register a memory range.  Inserts in sorted order (by `base`).
pub fn register(base: VirtAddr, len: usize) {
    let region = Region { base, end: VirtAddr::new(base.as_usize() + len) };
    let mut t = REGIONS.lock();
    let pos = t.partition_point(|r| r.base < region.base);
    t.insert(pos, region);
}

/// Drop a previously-registered range.
pub fn deregister(base: VirtAddr) {
    let mut t = REGIONS.lock();
    let pos = t.partition_point(|r| r.base < base);
    if pos < t.len() && t[pos].base == base { t.remove(pos); }
}

/// Try to satisfy a `#PF` at `va`.  Returns `Some(())` if a mapping was
/// installed and the instruction may retry; `None` otherwise.
pub fn try_handle(va: VirtAddr, _flags: crate::arch::x86_64::exception::PfFlags) -> Option<()> {
    if !lookup(va) { return None; }

    let frame = frame_alloc::alloc_frame()?;
    zero_frame(frame.0);

    let page_va = VirtAddr::new(va.as_usize() & !0xFFF);
    let flags = PageFlags::WRITABLE | PageFlags::NO_EXECUTE;
    unsafe { page_table::map_4k(page_va, frame.0, flags); }

    FAULTED_IN.fetch_add(1, Ordering::Relaxed);
    Some(())
}

/// Binary-search for `va` in the sorted region list.  Returns whether
/// any region contains `va`.
fn lookup(va: VirtAddr) -> bool {
    let t = REGIONS.lock();
    // partition_point(p) returns the first index where p is false.
    // Since regions are sorted by base, find the rightmost region whose
    // base <= va, then check the inclusive end.
    let pos = t.partition_point(|r| r.base <= va);
    if pos == 0 { return false; }
    let r = &t[pos - 1];
    va >= r.base && va < r.end
}

#[inline]
fn zero_frame(p: PhysAddr) {
    let v = phys_to_virt(p).as_mut_ptr::<u64>();
    for i in 0..(4096 / 8) {
        unsafe { core::ptr::write_volatile(v.add(i), 0); }
    }
}

#[inline] pub fn faulted_in() -> usize { FAULTED_IN.load(Ordering::Relaxed) }

/// Telemetry — currently registered region count.
pub fn region_count() -> usize { REGIONS.lock().len() }
