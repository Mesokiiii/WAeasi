//! virtio-blk request layout (VIRTIO 1.1 § 5.2.6).
//!
//! Each request is a 3-buffer chain:
//!   1. RO header `RequestHeader` (16 bytes).
//!   2. RO/WO data buffer (read = WO, write = RO).
//!   3. WO single-byte status.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum OpType {
    Read    = 0,
    Write   = 1,
    Flush   = 4,
    Discard = 11,
}

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct RequestHeader {
    pub op_type: u32,
    pub reserved: u32,
    pub sector:  u64, // LBA in 512-byte sectors
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Status { Ok = 0, IoErr = 1, Unsupp = 2 }
