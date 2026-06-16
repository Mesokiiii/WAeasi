//! Wasm linear-memory arena.
//!
//! Every Wasm instance gets its own *logical* linear memory, but **all** of
//! them are carved out of one contiguous virtual range — that is the SAS.
//! Isolation is provided by Wasm bounds-check instructions, not by paging.
//!
//! Each linear memory is page-aligned (64 KiB Wasm page) and placed inside
//! a 64 GiB arena starting at `ARENA_BASE`.  We hand out slices via a simple
//! bump pointer; freeing is a no-op for now (later: free list).
use core::sync::atomic::{AtomicUsize, Ordering};

use super::address::VirtAddr;

pub const ARENA_BASE: usize = 0x0000_0010_0000_0000; // 64 GiB
pub const ARENA_SIZE: usize = 64 * 1024 * 1024 * 1024;
pub const WASM_PAGE:  usize = 64 * 1024;

static NEXT: AtomicUsize = AtomicUsize::new(ARENA_BASE);

/// Handle to a linear memory region.
#[derive(Copy, Clone, Debug)]
pub struct LinearMemory {
    pub base:  VirtAddr,
    pub pages: u32,
}

impl LinearMemory {
    pub fn len(&self) -> usize { self.pages as usize * WASM_PAGE }
    pub fn is_empty(&self) -> bool { self.pages == 0 }

    /// SAFETY: caller must ensure the slice doesn't escape the instance.
    pub unsafe fn as_slice_mut(&self) -> &'static mut [u8] {
        core::slice::from_raw_parts_mut(self.base.as_mut_ptr::<u8>(), self.len())
    }
}

/// Initialize the arena (currently a no-op — bump pointer is enough).
pub fn init() {
    log::info!("[linear_mem] SAS arena {:#x} .. {:#x}",
               ARENA_BASE, ARENA_BASE + ARENA_SIZE);
}

/// Allocate `pages` Wasm pages (64 KiB each).
pub fn allocate(pages: u32) -> Option<LinearMemory> {
    let bytes = pages as usize * WASM_PAGE;
    let base  = NEXT.fetch_add(bytes, Ordering::Relaxed);
    if base + bytes > ARENA_BASE + ARENA_SIZE {
        return None;
    }
    Some(LinearMemory { base: VirtAddr::new(base), pages })
}

/// Grow `mem` by `delta` pages, returning the previous size in pages.
/// SAS allows in-place growth as long as the next region wasn't allocated.
pub fn grow(_mem: &mut LinearMemory, _delta: u32) -> Option<u32> {
    // Stage 1: not implemented — modules must declare max size up front.
    None
}
