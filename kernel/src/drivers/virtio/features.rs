//! Virtio feature-negotiation bits.
//!
//! Each device advertises a 64-bit feature mask; the driver reads it,
//! computes the intersection with what it supports, and writes back.
//! The device then exposes only that intersection.
use bitflags::bitflags;

bitflags! {
    /// Common to every virtio device (high 32 bits).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CommonFeatures: u64 {
        const ANY_LAYOUT     = 1 << 27;
        const RING_INDIRECT  = 1 << 28;
        const RING_EVENT_IDX = 1 << 29;
        const VERSION_1      = 1 << 32;
        const ACCESS_PLATFORM = 1 << 33;
        const RING_PACKED    = 1 << 34;
    }
}

bitflags! {
    /// virtio-net specific (low 32 bits).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NetFeatures: u64 {
        const CSUM           = 1 << 0;
        const GUEST_CSUM     = 1 << 1;
        const MAC            = 1 << 5;
        const STATUS         = 1 << 16;
        const MQ             = 1 << 22;
    }
}

bitflags! {
    /// virtio-blk specific (low 32 bits).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BlkFeatures: u64 {
        const BARRIER  = 1 << 0;
        const SIZE_MAX = 1 << 1;
        const SEG_MAX  = 1 << 2;
        const GEOMETRY = 1 << 4;
        const RO       = 1 << 5;
        const BLK_SIZE = 1 << 6;
        const FLUSH    = 1 << 9;
        const TOPOLOGY = 1 << 10;
        const CONFIG_WCE = 1 << 11;
        const DISCARD  = 1 << 13;
    }
}

/// Compute negotiated features.  We always require `VERSION_1` from the
/// device — modern (1.0+) interface only.
pub fn negotiate(device: u64, supported: u64) -> u64 {
    let common = CommonFeatures::VERSION_1.bits();
    let isect = device & supported;
    if isect & common != common {
        log::error!("[virtio] device lacks VERSION_1, refusing");
        return 0;
    }
    isect
}
