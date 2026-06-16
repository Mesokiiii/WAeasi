//! HPET (High Precision Event Timer) — used to **calibrate** the LAPIC
//! tick rate at boot.
//!
//! HPET frequency is fixed and reported in `GENERAL_CAPS[63:32]` as
//! femtoseconds-per-tick.  We measure how many LAPIC bus cycles tick in
//! `N` HPET periods and divide.
//!
//! The HPET base address comes from the ACPI HPET table (parsed in
//! stage 3).  Until then we accept it as a parameter so the function
//! is easy to unit-test offline.
use core::ptr::{read_volatile, write_volatile};

use crate::memory::address::PhysAddr;
use crate::memory::paging;

const REG_GENERAL_CAPS:   usize = 0x000;
const REG_GENERAL_CONFIG: usize = 0x010;
const REG_MAIN_COUNTER:   usize = 0x0F0;

pub struct Hpet { mmio: *mut u8 }
unsafe impl Send for Hpet {}
unsafe impl Sync for Hpet {}

impl Hpet {
    pub fn new(base_phys: PhysAddr) -> Self {
        let v = paging::map_mmio(base_phys, 0x400);
        Self { mmio: v as *mut u8 }
    }

    pub fn period_femto(&self) -> u32 {
        unsafe { (read_volatile(self.mmio.add(REG_GENERAL_CAPS) as *const u64) >> 32) as u32 }
    }

    pub fn enable(&self) {
        unsafe {
            let cfg_ptr = self.mmio.add(REG_GENERAL_CONFIG) as *mut u64;
            write_volatile(cfg_ptr, read_volatile(cfg_ptr) | 1);
        }
    }

    pub fn counter(&self) -> u64 {
        unsafe { read_volatile(self.mmio.add(REG_MAIN_COUNTER) as *const u64) }
    }

    /// Spin-wait `ns` nanoseconds.
    pub fn delay_ns(&self, ns: u64) {
        let period = self.period_femto() as u64; // fs per tick
        if period == 0 { return; }
        let target_ticks = (ns * 1_000_000) / period;
        let start = self.counter();
        while self.counter().wrapping_sub(start) < target_ticks {
            core::hint::spin_loop();
        }
    }
}
