//! Bootloader integration.
//!
//! WAeasi accepts two boot protocols at compile-time:
//!   * **Multiboot 2** (GRUB/Limine) — `multiboot2` submodule.
//!   * **Limine v6** native        — handled in stage 3.
//!
//! The first dword the bootloader hands us is a *magic*; from it we
//! discriminate which parser to use.  After parsing, we hand the
//! kernel a normalized `BootInfo` struct (free of bootloader specifics).
pub mod memmap;
pub mod multiboot2;

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct BootInfo {
    pub mem_regions: Vec<memmap::Region>,
    pub cmdline:     &'static str,
    pub modules:     Vec<BootModule>,
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
    if let Some(bi) = multiboot2::try_parse(info_ptr) {
        log::info!("[boot] multiboot2: {} regions, cmdline='{}', {} modules",
                   bi.mem_regions.len(), bi.cmdline, bi.modules.len());
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
    }
}
