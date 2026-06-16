//! Available + used ring layouts.
//!
//! Spec: VIRTIO 1.1 § 2.6.6 / § 2.6.8.  Both rings live next to the
//! descriptor table in a single physically-contiguous virtqueue blob.

#[repr(C)]
pub struct AvailRing {
    pub flags:    u16,
    pub idx:      u16,
    pub ring:     [u16; 0],   // `[u16; queue_size]` — flexible array
    pub used_evt: u16,        // VIRTIO_F_EVENT_IDX
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct UsedElem {
    pub id:  u32,    // descriptor index
    pub len: u32,    // bytes the device wrote
}

#[repr(C)]
pub struct UsedRing {
    pub flags:     u16,
    pub idx:       u16,
    pub ring:      [UsedElem; 0],   // `[UsedElem; queue_size]`
    pub avail_evt: u16,
}

/// Compute the on-memory layout of a virtqueue of `qsz` entries.
/// Returns offsets relative to the start of the virtqueue blob.
pub struct Layout {
    pub desc_off:   usize,
    pub avail_off:  usize,
    pub used_off:   usize,
    pub total_size: usize,
}

pub fn layout_for(qsz: usize) -> Layout {
    use core::mem::{align_of, size_of};
    let desc_size  = qsz * 16;                 // sizeof(Descriptor)
    let avail_size = 6 + qsz * 2;              // flags + idx + ring + used_evt
    let used_align = align_of::<UsedElem>() as usize; // 8
    // pad after avail to 4 (used must align 4 in legacy, 8 modern)
    let avail_end  = desc_size + avail_size;
    let used_off   = (avail_end + used_align - 1) & !(used_align - 1);
    let used_size  = 6 + qsz * size_of::<UsedElem>();
    Layout {
        desc_off:   0,
        avail_off:  desc_size,
        used_off,
        total_size: used_off + used_size,
    }
}
