//! ACPI System Description Table — common 36-byte header shared by every
//! ACPI table (RSDT, XSDT, MADT, HPET, MCFG, ...).
use crate::memory::address::PhysAddr;
use crate::memory::paging::phys_to_virt;

#[repr(C, packed)]
pub struct SdtHeader {
    pub signature:        [u8; 4],
    pub length:           u32,
    pub revision:         u8,
    pub checksum:         u8,
    pub oem_id:           [u8; 6],
    pub oem_table_id:     [u8; 8],
    pub oem_revision:     u32,
    pub creator_id:       u32,
    pub creator_revision: u32,
}

pub const HEADER_LEN: usize = core::mem::size_of::<SdtHeader>();

impl SdtHeader {
    pub fn length_usize(&self) -> usize {
        let l = self.length;
        l as usize
    }

    /// True iff the byte-sum of the whole table (header + body) is zero.
    pub fn validate(&self) -> bool {
        let len = self.length_usize();
        if len < HEADER_LEN { return false; }
        unsafe {
            let bytes = core::slice::from_raw_parts(self as *const Self as *const u8, len);
            bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b)) == 0
        }
    }
}

/// Read just the 4-byte signature at a physical address — used by the
/// XSDT walk before deciding whether to map the rest.
pub fn signature_at(phys: PhysAddr) -> Option<[u8; 4]> {
    let va = phys_to_virt(phys);
    let p  = va.as_ptr::<SdtHeader>();
    let header = unsafe { &*p };
    if header.validate() { Some(header.signature) } else { None }
}

/// Borrow an ACPI table at `phys` as a raw byte slice (header + body).
pub fn slice_at(phys: PhysAddr) -> Option<&'static [u8]> {
    let va = phys_to_virt(phys);
    let p  = va.as_ptr::<SdtHeader>();
    let header = unsafe { &*p };
    if !header.validate() { return None; }
    Some(unsafe { core::slice::from_raw_parts(p as *const u8, header.length_usize()) })
}
