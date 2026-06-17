//! Multiboot2 boot information parser + kernel header.
//!
//! Spec: https://www.gnu.org/software/grub/manual/multiboot2/multiboot.html
//!
//! Header bytes are emitted into `.multiboot_header` so a Multiboot2-aware
//! bootloader (GRUB 2 / Limine in MB2 mode) finds them in the first 32 KiB
//! of the ELF.  The header is **end-aligned** to 8 bytes per spec.
//!
//! **No-alloc**: parser writes regions/modules into the static buffers
//! exposed by `super::regions_buf()` / `super::modules_buf()`.

use super::memmap::{self, Kind, Region};
use super::{
    BootModule, MAX_MODULES, MAX_REGIONS,
    modules_buf, regions_buf, set_modules_len, set_regions_len,
};

const MAGIC_KERNEL: u32 = 0xE85250D6;
const MAGIC_BOOTLOADER: u32 = 0x36D76289;
const ARCH_I386: u32 = 0;

const TAG_END:        u16 = 0;
const TAG_CMDLINE:    u16 = 1;
const TAG_MODULE:     u16 = 3;
const TAG_BASIC_MEM:  u16 = 4;
const TAG_MMAP:       u16 = 6;

/// Multiboot2 header.
///
/// Currently NOT installed in `.multiboot_header` because the boot
/// trampoline at `arch/x86_64/boot32.rs` publishes a Multiboot1 header
/// in that section instead — `qemu -kernel` only honours Multiboot1.
/// When a Multiboot2-capable loader (GRUB / Limine MB2) is reintroduced,
/// re-add `#[link_section = ".multiboot_header"]` and `#[used]` here.
#[repr(C, align(8))]
#[allow(dead_code)]
struct Header {
    magic:    u32,
    arch:     u32,
    length:   u32,
    checksum: u32,
    end_tag:  [u32; 2], // type=0, size=8
}

#[allow(dead_code)]
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

/// Try parsing the MB2 info struct at `addr`.  Returns `true` on
/// success and writes into the static buffers; `false` means it's not
/// an MB2 info struct (caller falls through).
pub fn try_parse(addr: usize, cmdline_out: &mut &'static str) -> bool {
    if addr == 0 || addr & 7 != 0 {
        return false;
    }
    let total_size = unsafe { core::ptr::read(addr as *const u32) } as usize;
    if !(8..0x10_0000).contains(&total_size) {
        return false;
    }

    let regions = regions_buf();
    let modules = modules_buf();
    let mut nr = 0;
    let mut nm = 0;

    let mut p = addr + 8;
    let end = addr + total_size;
    while p < end {
        let ttype = unsafe { core::ptr::read(p as *const u16) };
        let tsize = unsafe { core::ptr::read((p + 4) as *const u32) } as usize;
        if tsize < 8 { break; }
        match ttype {
            TAG_CMDLINE => *cmdline_out = read_cstr(p + 8),
            TAG_MODULE  => {
                if nm < MAX_MODULES {
                    modules[nm] = parse_module(p, tsize);
                    nm += 1;
                }
            }
            TAG_MMAP    => nr = parse_mmap_into(p, tsize, regions),
            TAG_BASIC_MEM | _ => {}
        }
        if ttype == TAG_END { break; }
        p += (tsize + 7) & !7; // 8-byte align
    }

    let nr = memmap::normalize(regions, nr);
    set_regions_len(nr);
    set_modules_len(nm);
    true
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

fn parse_mmap_into(p: usize, tsize: usize, out: &mut [Region]) -> usize {
    let entry_size = unsafe { core::ptr::read((p + 8)  as *const u32) } as usize;
    let _entry_ver = unsafe { core::ptr::read((p + 12) as *const u32) };
    let mut q = p + 16;
    let end = p + tsize;
    let mut n = 0;
    while q + entry_size <= end && n < MAX_REGIONS {
        let base = unsafe { core::ptr::read(q as *const u64) } as usize;
        let len  = unsafe { core::ptr::read((q + 8) as *const u64) } as usize;
        let kind = unsafe { core::ptr::read((q + 16) as *const u32) };
        out[n] = Region {
            start: base,
            end:   base + len,
            kind:  match kind {
                1 => Kind::Usable,
                3 => Kind::AcpiReclaim,
                4 => Kind::AcpiNvs,
                5 => Kind::BadRam,
                _ => Kind::Reserved,
            },
        };
        n += 1;
        q += entry_size;
    }
    n
}

/// Just to silence "unused" warnings if a downstream caller uses the
/// constants for diagnostic logging.
pub const fn bootloader_magic() -> u32 { MAGIC_BOOTLOADER }
