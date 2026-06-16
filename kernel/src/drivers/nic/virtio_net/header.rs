//! virtio-net packet header (12 bytes when no extensions are negotiated).
//!
//! Spec: VIRTIO 1.1 § 5.1.6.  Each TX/RX buffer is prefixed by this
//! header.  In stage 2 we negotiate the minimal feature set so most
//! fields stay zero.

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct VirtioNetHdr {
    pub flags:           u8,
    pub gso_type:        u8,
    pub hdr_len:         u16,
    pub gso_size:        u16,
    pub csum_start:      u16,
    pub csum_offset:     u16,
    pub num_buffers:     u16,
}

pub const HDR_LEN: usize = core::mem::size_of::<VirtioNetHdr>();

/// gso_type values.
pub const GSO_NONE: u8 = 0;

/// flag values.
pub const F_NEEDS_CSUM:   u8 = 1;
pub const F_DATA_VALID:   u8 = 2;
