//! Virtqueue descriptor table.
//!
//! Spec: VIRTIO 1.1 § 2.6.5.
//!
//! Each `Descriptor` is 16 bytes; the table is contiguous and aligned to
//! 16 bytes.  We store *physical* addresses — the device sees only
//! physical memory.
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DescFlags: u16 {
        /// Buffer continues via `next`.
        const NEXT     = 1 << 0;
        /// Device may *write* into this buffer.
        const WRITE    = 1 << 1;
        /// `addr` points to another descriptor table (indirect).
        const INDIRECT = 1 << 2;
    }
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug)]
pub struct Descriptor {
    pub addr:  u64,    // physical address of the buffer
    pub len:   u32,    // buffer length in bytes
    pub flags: u16,
    pub next:  u16,    // next-descriptor index when NEXT is set
}

impl Descriptor {
    pub const ZERO: Self = Self { addr: 0, len: 0, flags: 0, next: 0 };

    pub fn read_only(addr: u64, len: u32, next: Option<u16>) -> Self {
        let mut f = DescFlags::empty();
        if next.is_some() { f |= DescFlags::NEXT; }
        Self { addr, len, flags: f.bits(), next: next.unwrap_or(0) }
    }

    pub fn write_only(addr: u64, len: u32, next: Option<u16>) -> Self {
        let mut f = DescFlags::WRITE;
        if next.is_some() { f |= DescFlags::NEXT; }
        Self { addr, len, flags: f.bits(), next: next.unwrap_or(0) }
    }
}
