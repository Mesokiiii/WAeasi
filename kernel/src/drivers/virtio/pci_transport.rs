//! Modern (1.0+) virtio-PCI transport.
//!
//! Stage 2 implements the modern (PCI capabilities) layout, not the
//! legacy I/O port one — every virtio implementation since 2014
//! supports modern, and it's measurably faster (MMIO + 64-bit BARs).
//!
//! Spec: VIRTIO 1.1 § 4.1.4.  Each capability lives in PCI config space
//! and points at one of:
//!   * common config           (cap_type = 1)
//!   * notification structure  (cap_type = 2)
//!   * ISR status              (cap_type = 3)
//!   * device-specific config  (cap_type = 4)
//!   * PCI configuration       (cap_type = 5)
use core::ptr::{read_volatile, write_volatile};

use crate::memory::address::PhysAddr;
use crate::memory::paging;

/// Subset of the common-config region we actually use.
#[repr(C)]
pub struct CommonConfig {
    pub device_feature_select: u32,
    pub device_feature:        u32,
    pub driver_feature_select: u32,
    pub driver_feature:        u32,
    pub msix_config:           u16,
    pub num_queues:            u16,
    pub device_status:         u8,
    pub config_generation:     u8,
    pub queue_select:          u16,
    pub queue_size:            u16,
    pub queue_msix_vector:     u16,
    pub queue_enable:          u16,
    pub queue_notify_off:      u16,
    pub queue_desc:            u64,
    pub queue_driver:          u64,
    pub queue_device:          u64,
}

pub mod status {
    pub const ACKNOWLEDGE: u8 = 1;
    pub const DRIVER:      u8 = 2;
    pub const DRIVER_OK:   u8 = 4;
    pub const FEATURES_OK: u8 = 8;
    pub const FAILED:      u8 = 128;
}

/// Map a virtio common-config region from its PCI BAR physical address.
pub fn map_common(bar_phys: PhysAddr) -> *mut CommonConfig {
    let v = paging::map_mmio(bar_phys, 1024);
    v as *mut CommonConfig
}

/// Negotiate features and reach `DRIVER_OK`.  Returns the negotiated
/// 64-bit feature mask.
pub unsafe fn bring_up(cc: *mut CommonConfig, supported: u64) -> u64 {
    write_volatile(&mut (*cc).device_status, 0);          // reset
    write_volatile(&mut (*cc).device_status, status::ACKNOWLEDGE);
    write_volatile(&mut (*cc).device_status,
                   status::ACKNOWLEDGE | status::DRIVER);

    // Read full 64-bit device features.
    write_volatile(&mut (*cc).device_feature_select, 0);
    let lo = read_volatile(&(*cc).device_feature) as u64;
    write_volatile(&mut (*cc).device_feature_select, 1);
    let hi = read_volatile(&(*cc).device_feature) as u64;
    let device = (hi << 32) | lo;

    let negotiated = super::features::negotiate(device, supported);

    // Write back driver-feature.
    write_volatile(&mut (*cc).driver_feature_select, 0);
    write_volatile(&mut (*cc).driver_feature, negotiated as u32);
    write_volatile(&mut (*cc).driver_feature_select, 1);
    write_volatile(&mut (*cc).driver_feature, (negotiated >> 32) as u32);

    write_volatile(&mut (*cc).device_status,
                   status::ACKNOWLEDGE | status::DRIVER | status::FEATURES_OK);

    let s = read_volatile(&(*cc).device_status);
    if s & status::FEATURES_OK == 0 {
        log::error!("[virtio_pci] device rejected features");
        write_volatile(&mut (*cc).device_status, status::FAILED);
        return 0;
    }
    write_volatile(&mut (*cc).device_status,
                   s | status::DRIVER_OK);
    negotiated
}
