//! Normalized memory map + range arithmetic.
//!
//! The bootloader presents wildly different formats — Multiboot2 has
//! E820-style regions, Limine has its own.  We normalize all of them
//! into a sorted slice here.
//!
//! **No-alloc**: this module runs *before* `memory::init`, so it must
//! not touch the heap.  Storage is provided by the caller as a
//! `&mut [Region]` slice, and `normalize` sorts/merges in-place,
//! returning the canonical length.

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
    pub const EMPTY: Region = Region {
        start: 0, end: 0, kind: Kind::Reserved,
    };

    pub fn len(&self) -> usize { self.end.saturating_sub(self.start) }
    pub fn is_usable(&self) -> bool { matches!(self.kind, Kind::Usable) }
}

/// Sort `regions[..len]` by `start`, then merge adjacent runs of the
/// same kind in-place.  Returns the new (possibly smaller) length.
///
/// Uses an O(N²) insertion sort to avoid recursion / stack growth in
/// the no-alloc early-boot context; `len` is bounded by the caller's
/// buffer size (typically 64).
pub fn normalize(regions: &mut [Region], mut len: usize) -> usize {
    // 1. Insertion sort by `start`.
    for i in 1..len {
        let mut j = i;
        while j > 0 && regions[j - 1].start > regions[j].start {
            regions.swap(j - 1, j);
            j -= 1;
        }
    }
    // 2. In-place merge of adjacent same-kind regions.
    if len <= 1 { return len; }
    let mut w = 0;
    for r in 1..len {
        let cur = regions[r];
        let last = regions[w];
        if last.kind == cur.kind && last.end >= cur.start {
            regions[w].end = last.end.max(cur.end);
        } else {
            w += 1;
            regions[w] = cur;
        }
    }
    len = w + 1;
    len
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
