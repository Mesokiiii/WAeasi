//! Limine boot protocol integration.
//!
//! Limine leaves the CPU in long mode with the kernel mapped at its
//! linker VMA (higher half) and the first 4 GiB identity-mapped — the
//! exact contract `arch/x86_64/boot.rs::_start` expects.
//!
//! Boot info is requested through statically-laid-out structures placed
//! in dedicated `.requests` linker sections.  Limine scans those at load
//! time, fills in the response pointers, then jumps to the kernel.
//!
//! The two markers (start / end) bound the request region so Limine can
//! discover requests without parsing the whole ELF.

use limine::memmap::{
    MEMMAP_ACPI_NVS, MEMMAP_ACPI_RECLAIMABLE, MEMMAP_BAD_MEMORY,
    MEMMAP_BOOTLOADER_RECLAIMABLE, MEMMAP_EXECUTABLE_AND_MODULES,
    MEMMAP_FRAMEBUFFER, MEMMAP_RESERVED, MEMMAP_USABLE,
};
use limine::request::{HhdmRequest, MemmapRequest};
use limine::{BaseRevision, RequestsEndMarker, RequestsStartMarker};

use super::memmap::{self, Kind, Region};
use super::{MAX_REGIONS, regions_buf, set_modules_len, set_regions_len};

#[used]
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::with_revision(2);

#[used]
#[link_section = ".requests"]
static MEMORY_MAP: MemmapRequest = MemmapRequest::new();

#[used]
#[link_section = ".requests"]
static HHDM: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests_start_marker"]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[link_section = ".requests_end_marker"]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

/// True when Limine actually populated the responses.
pub fn is_limine_boot() -> bool {
    BASE_REVISION.is_supported() && MEMORY_MAP.response().is_some()
}

/// Populate the static region buffer from Limine's responses.  Returns
/// `true` on success, `false` if Limine did not load us (caller falls
/// through to other protocols).
///
/// **No-alloc** — writes directly into `boot::REGIONS_BUF`.
pub fn try_parse(hhdm_offset: &mut u64) -> bool {
    if !is_limine_boot() {
        return false;
    }

    let Some(mmap) = MEMORY_MAP.response() else { return false };
    *hhdm_offset = HHDM.response().map(|r| r.offset).unwrap_or(0);

    let buf = regions_buf();
    let mut n = 0;
    for e in mmap.entries() {
        if n >= MAX_REGIONS { break; }
        buf[n] = Region {
            start: e.base as usize,
            end:   (e.base + e.length) as usize,
            kind:  classify(e.type_),
        };
        n += 1;
    }
    n = memmap::normalize(buf, n);
    set_regions_len(n);
    set_modules_len(0);
    true
}

fn classify(t: u64) -> Kind {
    match t {
        MEMMAP_USABLE                 => Kind::Usable,
        MEMMAP_RESERVED               => Kind::Reserved,
        MEMMAP_ACPI_RECLAIMABLE       => Kind::AcpiReclaim,
        MEMMAP_ACPI_NVS               => Kind::AcpiNvs,
        MEMMAP_BAD_MEMORY             => Kind::BadRam,
        MEMMAP_BOOTLOADER_RECLAIMABLE => Kind::BootloaderReclaim,
        MEMMAP_EXECUTABLE_AND_MODULES => Kind::KernelAndModules,
        MEMMAP_FRAMEBUFFER            => Kind::Framebuffer,
        _                             => Kind::Reserved,
    }
}
