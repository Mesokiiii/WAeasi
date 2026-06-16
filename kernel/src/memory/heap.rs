//! Kernel heap — backed by `linked_list_allocator`.
//!
//! Stage 1 lives entirely inside the bootloader-mapped direct map: we
//! grab a 16 MiB physically-contiguous region from the frame allocator
//! and hand it to `LockedHeap::init`.  All allocations are then served
//! from RAM that is already backed by physical frames + page-table
//! entries (the bootloader's direct map).
//!
//! Stage 2 will switch to lazy demand-paging: heap range is reserved up
//! front, frames materialize on first touch via #PF.
use core::sync::atomic::{AtomicUsize, Ordering};
use linked_list_allocator::LockedHeap;

use super::frame_alloc;
use super::paging::phys_to_virt;

pub const HEAP_SIZE: usize = 16 * 1024 * 1024;
const FRAMES: usize = HEAP_SIZE / 4096;

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

static HEAP_START: AtomicUsize = AtomicUsize::new(0);

/// One-shot heap initialization.  Allocates `FRAMES` contiguous frames
/// from the bump allocator (bump returns ascending addresses, so the
/// region is naturally contiguous in stage 1).
pub fn init() {
    let first = frame_alloc::alloc_frame()
        .expect("not enough RAM to bootstrap kernel heap");
    let start_v = phys_to_virt(first.0).as_usize();

    // Reserve the rest of the contiguous range.
    for _ in 1..FRAMES {
        frame_alloc::alloc_frame()
            .expect("heap reservation failed mid-stream");
    }

    HEAP_START.store(start_v, Ordering::Release);
    unsafe { ALLOCATOR.lock().init(start_v as *mut u8, HEAP_SIZE); }
    log::info!("[heap] {} MiB @ {:#x}", HEAP_SIZE / (1024 * 1024), start_v);
}

pub fn used_bytes() -> usize { ALLOCATOR.lock().used() }
pub fn free_bytes() -> usize { ALLOCATOR.lock().free() }
