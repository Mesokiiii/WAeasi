//! Allocator hook + OOM handler.
//!
//! The actual `#[global_allocator]` lives in `memory::heap` because the heap
//! lifecycle is tightly coupled to paging.  This file only installs the OOM
//! handler so the rest of the kernel doesn't have to care.
use core::alloc::Layout;

#[alloc_error_handler]
fn alloc_error(layout: Layout) -> ! {
    log::error!("OOM: failed to allocate {} bytes (align {})",
                layout.size(), layout.align());
    panic!("kernel allocator out of memory");
}
