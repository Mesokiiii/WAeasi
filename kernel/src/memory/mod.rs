//! Memory subsystem.
//!
//! Layout:
//! ```text
//! 0x0000_0010_0000_0000  Wasm linear-memory arena (SAS, demand-paged)
//! 0xFFFF_8000_0000_0000  direct map of physical RAM
//! 0xFFFF_FF00_0000_0000  MMIO arena (paging::map_mmio)
//! 0xFFFF_FFFF_8000_0000  kernel image
//! 0xFFFF_FFFF_C000_0000  kernel heap
//! ```
pub mod address;
pub mod buddy;
pub mod demand;
pub mod frame_alloc;
pub mod heap;
pub mod linear_mem;
pub mod page_table;
pub mod paging;

pub fn init(boot_info_ptr: usize) {
    frame_alloc::init(boot_info_ptr);
    paging::init();
    heap::init();
    linear_mem::init();
    log::info!("[memory] subsystem initialized");
}
