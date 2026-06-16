//! Root System Description Pointer (RSDP).
//!
//! BIOS path: scan the legacy `0xE0000..0xFFFFF` region for the
//! 8-byte signature `"RSD PTR "` on a 16-byte boundary; the first hit
//! whose checksum validates is the RSDP.
//!
//! UEFI path: the bootloader passes the address via the EFI Config
//! Table.  Stage 4 accepts it through `set_uefi`.
use core::sync::atomic::{AtomicUsize, Ordering};

#[repr(C, packed)]
pub struct Rsdp {
    pub signature:    [u8; 8],
    pub checksum:     u8,
    pub oem_id:       [u8; 6],
    pub revision:     u8,
    pub rsdt_addr:    u32,
    pub length:       u32,
    pub xsdt_addr:    u64,
    pub ext_checksum: u8,
    pub _reserved:    [u8; 3],
}

impl Rsdp {
    pub fn xsdt_addr(&self) -> u64 {
        let v = self.xsdt_addr;  // copy out of packed struct
        v
    }

    fn validate(&self) -> bool {
        if &self.signature != b"RSD PTR " { return false; }
        // ACPI 1.0 sums the first 20 bytes; ACPI 2.0+ sums the whole struct.
        unsafe {
            let bytes_v1 = core::slice::from_raw_parts(self as *const Self as *const u8, 20);
            if checksum(bytes_v1) != 0 { return false; }
            if self.revision >= 2 {
                let bytes_full = core::slice::from_raw_parts(self as *const Self as *const u8,
                                                              self.length as usize);
                if checksum(bytes_full) != 0 { return false; }
            }
        }
        true
    }
}

#[inline]
fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b))
}

static UEFI_RSDP: AtomicUsize = AtomicUsize::new(0);

/// Bootloader-supplied RSDP address (UEFI path).
pub fn set_uefi(addr: usize) { UEFI_RSDP.store(addr, Ordering::Release); }

/// Find the RSDP.  UEFI override takes precedence over BIOS scan.
pub fn find() -> Option<&'static Rsdp> {
    if let Some(p) = uefi_path() { return Some(p); }
    bios_scan()
}

fn uefi_path() -> Option<&'static Rsdp> {
    let addr = UEFI_RSDP.load(Ordering::Acquire);
    if addr == 0 { return None; }
    let r = unsafe { &*(addr as *const Rsdp) };
    if r.validate() { Some(r) } else { None }
}

fn bios_scan() -> Option<&'static Rsdp> {
    use crate::memory::paging::DIRECT_MAP_BASE;
    for off in (0xE0000..0xFFFFF).step_by(16) {
        let va = DIRECT_MAP_BASE + off;
        let candidate = unsafe { &*(va as *const Rsdp) };
        if candidate.validate() { return Some(candidate); }
    }
    None
}
