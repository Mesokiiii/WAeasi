//! Bootloader integration.
//!
//! WAeasi accepts two boot protocols at compile-time:
//!   * **Limine v9+ native** — preferred path; gives long mode + higher-half
//!     mapping that `_start` already expects.  See `boot::limine`.
//!   * **Multiboot 1** (GRUB / `qemu -kernel`)  — `multiboot2` submodule
//!     also handles MB2 if encountered.  Currently the runtime path is
//!     MB1, with an in-tree 32→64 trampoline at `arch::x86_64::boot32`.
//!
//! `parse()` tries protocols in priority order and returns the first that
//! matches.  All return a normalized [`BootInfo`] free of bootloader
//! specifics.
//!
//! ## No-alloc contract
//!
//! `parse()` runs **before** `memory::init`, i.e. before the global
//! allocator is up.  It therefore stores its results in static
//! buffers (`REGIONS_BUF`, `MODULES_BUF`) and exposes them as
//! `&'static [...]` slices.  Parsers must never touch the heap.
pub mod limine;
pub mod memmap;
pub mod multiboot2;

use core::sync::atomic::{AtomicUsize, Ordering};

/// Maximum number of memory regions we record.  Real machines rarely
/// expose more than ~30 entries.
pub const MAX_REGIONS: usize = 64;
/// Maximum bootloader-supplied modules.
pub const MAX_MODULES: usize = 16;

// SAFETY: only mutated inside `parse()` (single-threaded early boot,
// IRQs off, BSP only).  After `parse()` returns, the slices handed
// out via `BootInfo` are read-only for the rest of the kernel
// lifetime.
static mut REGIONS_BUF: [memmap::Region; MAX_REGIONS] =
    [memmap::Region::EMPTY; MAX_REGIONS];
static mut MODULES_BUF: [BootModule; MAX_MODULES] =
    [BootModule { start: 0, end: 0, name: "" }; MAX_MODULES];

static REGIONS_LEN: AtomicUsize = AtomicUsize::new(0);
static MODULES_LEN: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy)]
pub struct BootInfo {
    pub mem_regions: &'static [memmap::Region],
    pub cmdline:     &'static str,
    pub modules:     &'static [BootModule],
    /// Higher-half direct-map offset reported by the bootloader.
    /// Zero when running under a protocol that does not expose HHDM.
    pub hhdm_offset: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct BootModule {
    pub start: usize,
    pub end:   usize,
    pub name:  &'static str,
}

// ---------------------------------------------------------------------------
// Buffer helpers — used by individual parser submodules.
// ---------------------------------------------------------------------------

pub(crate) fn regions_buf() -> &'static mut [memmap::Region] {
    #[allow(static_mut_refs)]
    unsafe { &mut REGIONS_BUF[..] }
}
pub(crate) fn modules_buf() -> &'static mut [BootModule] {
    #[allow(static_mut_refs)]
    unsafe { &mut MODULES_BUF[..] }
}
pub(crate) fn set_regions_len(n: usize) { REGIONS_LEN.store(n, Ordering::Release); }
pub(crate) fn set_modules_len(n: usize) { MODULES_LEN.store(n, Ordering::Release); }

fn regions_slice() -> &'static [memmap::Region] {
    let n = REGIONS_LEN.load(Ordering::Acquire).min(MAX_REGIONS);
    #[allow(static_mut_refs)]
    unsafe { &REGIONS_BUF[..n] }
}
fn modules_slice() -> &'static [BootModule] {
    let n = MODULES_LEN.load(Ordering::Acquire).min(MAX_MODULES);
    #[allow(static_mut_refs)]
    unsafe { &MODULES_BUF[..n] }
}

/// Parse whatever the bootloader gave us at `info_ptr`.  Returns a
/// canonical [`BootInfo`].  When neither Limine nor Multiboot is
/// recognized the kernel still gets a minimal hardcoded layout so it
/// can boot for diagnostics.
pub fn parse(info_ptr: usize) -> BootInfo {
    log::info!("[boot] parse info_ptr={:#x}", info_ptr);

    let mut cmdline:     &'static str = "";
    let mut hhdm_offset: u64          = 0;

    if limine::try_parse(&mut hhdm_offset) {
        log::info!("[boot] limine: {} regions, hhdm={:#x}",
                   regions_slice().len(), hhdm_offset);
    } else if multiboot2::try_parse(info_ptr, &mut cmdline) {
        log::info!("[boot] multiboot2: {} regions, cmdline='{}', {} modules",
                   regions_slice().len(), cmdline, modules_slice().len());
    } else {
        log::warn!("[boot] no recognised bootinfo at {:#x}, using fallback layout",
                   info_ptr);
        let buf = regions_buf();
        buf[0] = memmap::Region {
            start: 16 * 1024 * 1024,
            end:   16 * 1024 * 1024 + 64 * 1024 * 1024,
            kind:  memmap::Kind::Usable,
        };
        set_regions_len(1);
        set_modules_len(0);
    }

    BootInfo {
        mem_regions: regions_slice(),
        cmdline,
        modules:     modules_slice(),
        hhdm_offset,
    }
}
