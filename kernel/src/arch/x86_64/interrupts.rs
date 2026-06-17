//! Hardware IRQ vector table (>= 32) and dispatch logic.
//!
//! In a Cloud-Native OS we only care about a handful of IRQs:
//!   * timer   — drives the async reactor's clock,
//!   * NIC     — drives socket wakers,
//!   * block   — drives storage wakers.
//!
//! Everything else gets a generic spurious-IRQ handler.
use core::sync::atomic::{AtomicU64, Ordering};

use super::apic;
use super::idt::{set, InterruptFrame};

/// Vector offsets — keep clear of CPU exceptions (0..32).
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum HwIrq {
    Timer    = 32,
    Keyboard = 33,
    Nic      = 43,
    Block    = 44,
    Spurious = 0xFF,
}

static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

/// Install hardware IRQ handlers into the live IDT.  Must run after
/// `idt::init` has loaded IDTR — entries are written by `idt::set`,
/// which mutates the same static IDT table directly.
pub fn register_hardware_irqs() {
    set(HwIrq::Timer    as u8, timer);
    set(HwIrq::Keyboard as u8, keyboard);
    set(HwIrq::Nic      as u8, nic);
    set(HwIrq::Block    as u8, block);
    set(HwIrq::Spurious as u8, spurious);
}

/// Globally enable IRQs after the IDT is loaded.
pub fn init() {
    register_hardware_irqs();
    super::cpu::enable_interrupts();
    log::debug!("[interrupts] hardware IRQs enabled");
}

/// Number of timer ticks since boot.
#[inline]
pub fn ticks() -> u64 { TIMER_TICKS.load(Ordering::Relaxed) }

extern "x86-interrupt" fn timer(_: InterruptFrame) {
    TIMER_TICKS.fetch_add(1, Ordering::Relaxed);
    crate::sched::reactor::on_timer_tick();
    apic::eoi();
}

extern "x86-interrupt" fn keyboard(_: InterruptFrame) {
    crate::drivers::console::on_keyboard_irq();
    apic::eoi();
}

extern "x86-interrupt" fn nic(_: InterruptFrame) {
    crate::drivers::nic::on_irq();
    apic::eoi();
}

extern "x86-interrupt" fn block(_: InterruptFrame) {
    crate::drivers::block::on_irq();
    apic::eoi();
}

extern "x86-interrupt" fn spurious(_: InterruptFrame) {
    log::trace!("[interrupts] spurious");
    apic::eoi();
}
