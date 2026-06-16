//! Paging — high-level API.
//!
//! Two responsibilities:
//!   1. Trust the bootloader-provided PML4 (which already maps RAM at
//!      `DIRECT_MAP_BASE` and the kernel image at `KERNEL_OFFSET`).
//!   2. Carve MMIO windows on demand inside `[MMIO_BASE, MMIO_END)` with
//!      uncached + NX flags, allocating intermediate page-table frames
//!      lazily.
use core::sync::atomic::{AtomicUsize, Ordering};

use super::address::{PhysAddr, VirtAddr, PAGE_SIZE_4K};
use super::page_table;

pub const DIRECT_MAP_BASE: usize = 0xFFFF_8000_0000_0000;
pub const MMIO_BASE:       usize = 0xFFFF_FF00_0000_0000;
pub const MMIO_END:        usize = 0xFFFF_FF80_0000_0000;

/// Lock-free bump pointer for the MMIO arena.
static MMIO_NEXT: AtomicUsize = AtomicUsize::new(MMIO_BASE);

bitflags::bitflags! {
    /// x86_64 page-table entry flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageFlags: u64 {
        const PRESENT      = 1 << 0;
        const WRITABLE     = 1 << 1;
        const USER         = 1 << 2;
        const WRITE_THRU   = 1 << 3;
        const NO_CACHE     = 1 << 4;
        const ACCESSED     = 1 << 5;
        const DIRTY        = 1 << 6;
        const HUGE         = 1 << 7;
        const GLOBAL       = 1 << 8;
        const NO_EXECUTE   = 1 << 63;
    }
}

/// One-shot init.  Stage 1 leaves the bootloader's tables intact and only
/// records the active PML4 for telemetry.
pub fn init() {
    log::info!("[paging] using bootloader-supplied PML4 (cr3 = {:?})",
               page_table::current_pml4_phys());
}

/// Map `len` bytes of MMIO starting at `phys` into the kernel address space
/// as RW + uncached + NX.  Returns the virtual base **including the same
/// in-page offset** as `phys`.
///
/// The function rounds the range up to 4 KiB pages and installs one PT
/// entry per page, allocating PML4 / PDPT / PD frames as needed.
pub fn map_mmio(phys: PhysAddr, len: usize) -> usize {
    let page  = PAGE_SIZE_4K as usize;
    let start = phys.as_usize() & !(page - 1);
    let end   = (phys.as_usize() + len + page - 1) & !(page - 1);
    let pages = (end - start) / page;

    let virt_base = MMIO_NEXT.fetch_add(pages * page, Ordering::Relaxed);
    assert!(virt_base + pages * page <= MMIO_END, "MMIO arena exhausted");

    let flags = PageFlags::WRITABLE | PageFlags::NO_CACHE | PageFlags::NO_EXECUTE;
    for i in 0..pages {
        let v = VirtAddr::new(virt_base + i * page);
        let p = PhysAddr::new(start + i * page);
        unsafe { page_table::map_4k(v, p, flags); }
    }

    let off = phys.as_usize() - start;
    log::debug!("[paging] mmio map {:?} len={:#x} -> {:#x}", phys, len, virt_base + off);
    virt_base + off
}

/// Translate a kernel virtual address to physical via the direct map.
pub fn virt_to_phys(v: VirtAddr) -> Option<PhysAddr> {
    let v = v.as_usize();
    if (DIRECT_MAP_BASE..MMIO_BASE).contains(&v) {
        Some(PhysAddr::new(v - DIRECT_MAP_BASE))
    } else {
        None
    }
}

/// Convert a physical address to its direct-map virtual address.
#[inline]
pub fn phys_to_virt(p: PhysAddr) -> VirtAddr {
    VirtAddr::new(DIRECT_MAP_BASE + p.as_usize())
}
