//! Multiboot2 boot information parser + kernel header.
//!
//! Spec: https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
//!
//! Header bytes are emitted into `.multiboot_header` so a Multiboot2-aware
//! bootloader (GRUB 2 / Limine in MB2 mode) finds them in the first 32 KiB
//! of the ELF.  The header is **end-aligned** to 8 bytes per spec.
use alloc::vec::Vec;

use super::memmap::{Kind, Region};
use super::{BootInfo, BootModule};

const MAGIC_KERNEL: u32 = 0xE85250D6;
const MAGIC_BOOTLOADER: u32 = 0x36D76289;
const ARCH_I386: u32 = 0;

const TAG_END:        u16 = 0;
const TAG_CMDLINE:    u16 = 1;
const TAG_MODULE:     u16 = 3;
const TAG_BASIC_MEM:  u16 = 4;
const TAG_MMAP:       u16 = 6;

/// Multiboot2 header.  `#[link_section = ".multiboot_header"]` parks it in
/// the first page so the bootloader can find it.
#[repr(C, align(8))]
struct Header {
    magic:    u32,
    arch:     u32,
    length:   u32,
    checksum: u32,
    end_tag:  [u32; 2], // type=0, size=8
}

#[link_section = ".multiboot_header"]
#[used]
static HEADER: Header = Header {
    magic:    MAGIC_KERNEL,
    arch:     ARCH_I386,
    length:   core::mem::size_of::<Header>() as u32,
    checksum: 0u32
        .wrapping_sub(MAGIC_KERNEL)
        .wrapping_sub(ARCH_I386)
        .wrapping_sub(core::mem::size_of::<Header>() as u32),
    end_tag:  [0, 8],
};

/// Try parsing the MB2 info struct at `addr`.  Returns `None` if the
/// magic (handed to us in `eax` by the bootloader) is wrong — but since
/// we don't have eax here, we sniff the first dword as a size sanity check.
pub fn try_parse(addr: usize) -> Option<BootInfo> {
    if addr == 0 || addr & 7 != 0 { return None; }
    let total_size = unsafe { core::ptr::read(addr as *const u32) } as usize;
    if !(8..0x10_0000).contains(&total_size) { return None; }

    let mut regions = Vec::new();
    let mut cmdline: &'static str = "";
    let mut modules = Vec::new();

    let mut p = addr + 8;
    let end = addr + total_size;
    while p < end {
        let ttype = unsafe { core::ptr::read(p as *const u16) };
        let tsize = unsafe { core::ptr::read((p + 4) as *const u32) } as usize;
        if tsize < 8 { break; }
        match ttype {
            TAG_CMDLINE  => cmdline = read_cstr(p + 8),
            TAG_MODULE   => modules.push(parse_module(p, tsize)),
            TAG_MMAP     => regions = parse_mmap(p, tsize),
            TAG_BASIC_MEM | _ => {}
        }
        if ttype == TAG_END { break; }
        p += (tsize + 7) & !7; // 8-byte align
    }

    Some(BootInfo {
        mem_regions: super::memmap::normalize(regions),
        cmdline,
        modules,
        hhdm_offset: 0,
    })
}

unsafe fn read_str(addr: usize, len: usize) -> &'static str {
    let s = core::slice::from_raw_parts(addr as *const u8, len);
    core::str::from_utf8(s).unwrap_or("")
}

fn read_cstr(addr: usize) -> &'static str {
    unsafe {
        let mut len = 0;
        while core::ptr::read((addr + len) as *const u8) != 0 { len += 1; }
        read_str(addr, len)
    }
}

fn parse_module(p: usize, _tsize: usize) -> BootModule {
    let start = unsafe { core::ptr::read((p + 8) as *const u32) } as usize;
    let end   = unsafe { core::ptr::read((p + 12) as *const u32) } as usize;
    let name  = read_cstr(p + 16);
    BootModule { start, end, name }
}

fn parse_mmap(p: usize, tsize: usize) -> Vec<Region> {
    let entry_size = unsafe { core::ptr::read((p + 8)  as *const u32) } as usize;
    let _entry_ver = unsafe { core::ptr::read((p + 12) as *const u32) };
    let mut entries = Vec::new();
    let mut q = p + 16;
    let end = p + tsize;
    while q + entry_size <= end {
        let base = unsafe { core::ptr::read(q as *const u64) } as usize;
        let len  = unsafe { core::ptr::read((q + 8) as *const u64) } as usize;
        let kind = unsafe { core::ptr::read((q + 16) as *const u32) };
        entries.push(Region {
            start: base,
            end:   base + len,
            kind:  match kind {
                1 => Kind::Usable,
                3 => Kind::AcpiReclaim,
                4 => Kind::AcpiNvs,
                5 => Kind::BadRam,
                _ => Kind::Reserved,
            },
        });
        q += entry_size;
    }
    entries
}

/// Just to silence "unused" warnings if a downstream caller uses the
/// constants for diagnostic logging.
pub const fn bootloader_magic() -> u32 { MAGIC_BOOTLOADER }
