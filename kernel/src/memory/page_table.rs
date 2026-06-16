//! 4-level x86_64 page-table primitives.
//!
//! This module is intentionally minimal: it exposes only what the kernel
//! needs to add fresh mappings to the bootloader-provided PML4 (carving
//! MMIO windows on demand).  Reading existing mappings is delegated to
//! `paging::virt_to_phys` via the direct map.
use core::sync::atomic::{AtomicU64, Ordering};

use super::address::{PhysAddr, VirtAddr, PAGE_SIZE_4K};
use super::frame_alloc;
use super::paging::{PageFlags, DIRECT_MAP_BASE};

/// Number of entries per table (always 512 on x86_64).
pub const TABLE_LEN: usize = 512;

/// Wrapper over a 64-bit page-table entry.
#[repr(transparent)]
pub struct Entry(AtomicU64);

impl Entry {
    pub fn load(&self) -> u64 { self.0.load(Ordering::Acquire) }
    pub fn store(&self, raw: u64) { self.0.store(raw, Ordering::Release); }
    pub fn is_present(&self) -> bool { self.load() & 1 != 0 }
    pub fn frame(&self) -> PhysAddr { PhysAddr::new((self.load() & 0x000F_FFFF_FFFF_F000) as usize) }
}

/// A 512-entry page table.
#[repr(C, align(4096))]
pub struct Table {
    entries: [Entry; TABLE_LEN],
}

impl Table {
    /// Map a freshly allocated physical frame to a kernel virtual address
    /// via the direct map (`DIRECT_MAP_BASE + phys`).
    pub unsafe fn from_phys_mut<'a>(p: PhysAddr) -> &'a mut Table {
        let v = DIRECT_MAP_BASE + p.as_usize();
        &mut *(v as *mut Table)
    }

    pub fn entry(&self, i: usize) -> &Entry { &self.entries[i] }
}

/// Decompose a virtual address into PML4 / PDPT / PD / PT indices.
#[inline]
pub fn indices(v: VirtAddr) -> [usize; 4] {
    let a = v.as_u64();
    [
        ((a >> 39) & 0x1FF) as usize, // PML4
        ((a >> 30) & 0x1FF) as usize, // PDPT
        ((a >> 21) & 0x1FF) as usize, // PD
        ((a >> 12) & 0x1FF) as usize, // PT
    ]
}

/// Read the active PML4 from CR3.
#[inline]
pub fn current_pml4_phys() -> PhysAddr {
    let cr3: u64;
    unsafe { core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem)) };
    PhysAddr::new((cr3 & 0x000F_FFFF_FFFF_F000) as usize)
}

/// Map a single 4 KiB page `virt -> phys` with `flags`.  Allocates intermediate
/// page-table levels from `frame_alloc` as needed.  All allocated frames are
/// zero-initialized via the direct map.
pub unsafe fn map_4k(virt: VirtAddr, phys: PhysAddr, flags: PageFlags) {
    let idx = indices(virt);
    let mut table = Table::from_phys_mut(current_pml4_phys());

    for level in 0..3 {
        let e = table.entry(idx[level]);
        let next_phys = if e.is_present() {
            e.frame()
        } else {
            let new = frame_alloc::alloc_frame().expect("OOM in page-table walk");
            zero_frame(new.0);
            let parent_flags = (PageFlags::PRESENT | PageFlags::WRITABLE).bits();
            e.store(new.0.as_u64() | parent_flags);
            new.0
        };
        table = Table::from_phys_mut(next_phys);
    }

    let leaf = table.entry(idx[3]);
    leaf.store(phys.as_u64() | flags.bits() | PageFlags::PRESENT.bits());
    invlpg(virt);
}

#[inline]
fn zero_frame(p: PhysAddr) {
    let v = (DIRECT_MAP_BASE + p.as_usize()) as *mut u64;
    unsafe {
        for i in 0..(PAGE_SIZE_4K as usize / 8) {
            core::ptr::write_volatile(v.add(i), 0);
        }
    }
}

#[inline]
fn invlpg(v: VirtAddr) {
    unsafe { core::arch::asm!("invlpg [{}]", in(reg) v.as_usize(), options(nostack)) };
}
