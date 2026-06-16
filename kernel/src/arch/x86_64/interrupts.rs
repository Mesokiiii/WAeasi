//! Hardware IRQ vector table (>= 32) and dispatch logic.
//!
//! In a Cloud-Native OS we only care about a handful of IRQs:
//!   * timer   — drives the async reactor's clock,
//!   * NIC     — drives socket wakers,
//!   * block   — drives storage wakers.
//!
//! Everything else gets a generic spurious-IRQ handler.
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

use super::apic;

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

/// Install hardware IRQ handlers into a freshly built IDT.
pub fn register_hardware_irqs(idt: &mut InterruptDescriptorTable) {
    idt[HwIrq::Timer    as u8].set_handler_fn(timer);
    idt[HwIrq::Keyboard as u8].set_handler_fn(keyboard);
    idt[HwIrq::Nic      as u8].set_handler_fn(nic);
    idt[HwIrq::Block    as u8].set_handler_fn(block);
    idt[HwIrq::Spurious as u8].set_handler_fn(spurious);
}

/// Globally enable IRQs after the IDT is loaded.
pub fn init() {
    super::cpu::enable_interrupts();
    log::debug!("[interrupts] hardware IRQs enabled");
}

/// Number of timer ticks since boot.
#[inline]
pub fn ticks() -> u64 { TIMER_TICKS.load(Ordering::Relaxed) }

extern "x86-interrupt" fn timer(_: InterruptStackFrame) {
    TIMER_TICKS.fetch_add(1, Ordering::Relaxed);
    crate::sched::reactor::on_timer_tick();
    apic::eoi();
}

extern "x86-interrupt" fn keyboard(_: InterruptStackFrame) {
    crate::drivers::console::on_keyboard_irq();
    apic::eoi();
}

extern "x86-interrupt" fn nic(_: InterruptStackFrame) {
    crate::drivers::nic::on_irq();
    apic::eoi();
}

extern "x86-interrupt" fn block(_: InterruptStackFrame) {
    crate::drivers::block::on_irq();
    apic::eoi();
}

extern "x86-interrupt" fn spurious(_: InterruptStackFrame) {
    log::trace!("[interrupts] spurious");
    apic::eoi();
}
