//! Bootloader integration.
//!
//! WAeasi accepts two boot protocols at compile-time:
//!   * **Limine v9+ native** — preferred path; gives long mode + higher-half
//!     mapping that `_start` already expects.  See `boot::limine`.
//!   * **Multiboot 2** (GRUB)  — `multiboot2` submodule.  Currently parses
//!     the info struct but expects an external 32→64 trampoline; reserved
//!     for the future, kept for protocol parity.
//!
//! `parse()` tries protocols in priority order and returns the first that
//! matches.  All return a normalized [`BootInfo`] free of bootloader
//! specifics.
pub mod limine;
pub mod memmap;
pub mod multiboot2;

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct BootInfo {
    pub mem_regions: Vec<memmap::Region>,
    pub cmdline:     &'static str,
    pub modules:     Vec<BootModule>,
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

/// Parse whatever the bootloader gave us at `info_ptr`.  Returns a
/// canonical `BootInfo`.  Errors fall back to a minimal hardcoded layout
/// so the kernel can still boot for diagnostics.
pub fn parse(info_ptr: usize) -> BootInfo {
    if let Some(bi) = limine::try_parse() {
        log::info!(
            "[boot] limine: {} regions, hhdm={:#x}",
            bi.mem_regions.len(), bi.hhdm_offset,
        );
        return bi;
    }
    if let Some(bi) = multiboot2::try_parse(info_ptr) {
        log::info!(
            "[boot] multiboot2: {} regions, cmdline='{}', {} modules",
            bi.mem_regions.len(), bi.cmdline, bi.modules.len(),
        );
        return bi;
    }
    log::warn!("[boot] no recognised bootinfo at {:#x}, using fallback", info_ptr);
    BootInfo {
        mem_regions: alloc::vec![memmap::Region {
            start: 16 * 1024 * 1024,
            end:   16 * 1024 * 1024 + 64 * 1024 * 1024,
            kind:  memmap::Kind::Usable,
        }],
        cmdline: "",
        modules: Vec::new(),
        hhdm_offset: 0,
    }
}
