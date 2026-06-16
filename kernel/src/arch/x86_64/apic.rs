//! Local APIC bring-up + EOI helper.
//!
//! Stage 1 wires the LAPIC timer in periodic mode at a known rate so
//! `clocks::NS_PER_TICK` stays consistent.  We pick **100 Hz** (10 ms per
//! tick) — fine resolution for a cloud workload, coarse enough to keep
//! the IRQ overhead negligible (<0.1 % CPU on a 3 GHz core).
//!
//! Calibration is approximate: the divisor / init-count combo below
//! assumes a typical 50 MHz LAPIC bus on QEMU/KVM.  Stage 2 will read
//! HPET to calibrate at boot.
use core::ptr::{read_volatile, write_volatile};
use spin::Once;

use crate::memory::address::PhysAddr;
use crate::memory::paging;

const APIC_BASE_PHYS: usize = 0xFEE0_0000;
const REG_EOI:        usize = 0x0B0;
const REG_SVR:        usize = 0x0F0;
const REG_LVT_TIMER:  usize = 0x320;
const REG_TIMER_INIT: usize = 0x380;
const REG_TIMER_DIV:  usize = 0x3E0;

/// Target tick rate — must match `wasi::preview2::clocks::NS_PER_TICK`.
pub const TICK_HZ: u32 = 100;

/// LAPIC timer init count for ~100 Hz @ 50 MHz bus, /16 divisor.
///   50_000_000 / 16 / 100 = 31_250
const TIMER_INIT_COUNT: u32 = 50_000_000 / 16 / TICK_HZ;
const TIMER_DIV_BY_16:  u32 = 0b011;
const TIMER_PERIODIC:   u32 = 1 << 17;
const TIMER_VECTOR:     u32 = 32;

static APIC: Once<Apic> = Once::new();

struct Apic { mmio: *mut u8 }
unsafe impl Send for Apic {}
unsafe impl Sync for Apic {}

impl Apic {
    #[inline]
    unsafe fn read(&self, off: usize) -> u32 {
        read_volatile(self.mmio.add(off) as *const u32)
    }
    #[inline]
    unsafe fn write(&self, off: usize, val: u32) {
        write_volatile(self.mmio.add(off) as *mut u32, val);
    }
}

/// Map LAPIC MMIO and configure the periodic timer.
pub fn init() {
    let virt = paging::map_mmio(PhysAddr::new(APIC_BASE_PHYS), 0x1000);
    let apic = APIC.call_once(|| Apic { mmio: virt as *mut u8 });

    unsafe {
        // Spurious-interrupt vector register: enable APIC + vector 0xFF.
        apic.write(REG_SVR, 0x1FF);
        // Periodic timer.
        apic.write(REG_TIMER_DIV, TIMER_DIV_BY_16);
        apic.write(REG_LVT_TIMER, TIMER_VECTOR | TIMER_PERIODIC);
        apic.write(REG_TIMER_INIT, TIMER_INIT_COUNT);
    }
    log::debug!("[apic] online @ {:p}, timer @ {} Hz", apic.mmio, TICK_HZ);
}

/// Signal end-of-interrupt for the current vector.
#[inline]
pub fn eoi() {
    if let Some(a) = APIC.get() {
        unsafe { a.write(REG_EOI, 0) }
    }
}
