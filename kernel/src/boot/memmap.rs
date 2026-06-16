//! Normalized memory map + range arithmetic.
//!
//! The bootloader presents wildly different formats — Multiboot2 has
//! E820-style regions, Limine has its own.  We normalize all of them
//! into a sorted `Vec<Region>` here.
use alloc::vec::Vec;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Usable,
    Reserved,
    AcpiReclaim,
    AcpiNvs,
    BadRam,
    BootloaderReclaim,
    KernelAndModules,
    Framebuffer,
}

#[derive(Copy, Clone, Debug)]
pub struct Region {
    pub start: usize,
    pub end:   usize,
    pub kind:  Kind,
}

impl Region {
    pub fn len(&self) -> usize { self.end.saturating_sub(self.start) }
    pub fn is_usable(&self) -> bool { matches!(self.kind, Kind::Usable) }
}

/// Sort + merge adjacent regions of the same kind.  After this the slice
/// is canonical and can be binary-searched.
pub fn normalize(mut v: Vec<Region>) -> Vec<Region> {
    v.sort_by_key(|r| r.start);
    let mut out: Vec<Region> = Vec::with_capacity(v.len());
    for r in v {
        if let Some(last) = out.last_mut() {
            if last.kind == r.kind && last.end >= r.start {
                last.end = last.end.max(r.end);
                continue;
            }
        }
        out.push(r);
    }
    out
}

/// Largest contiguous usable region — a quick way for early
/// `frame_alloc::init` to pick the biggest single chunk.
pub fn largest_usable(v: &[Region]) -> Option<Region> {
    v.iter().filter(|r| r.is_usable()).max_by_key(|r| r.len()).copied()
}

/// Total usable bytes summed across all regions.
pub fn total_usable(v: &[Region]) -> usize {
    v.iter().filter(|r| r.is_usable()).map(|r| r.len()).sum()
}
