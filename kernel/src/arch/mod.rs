//! Architecture abstraction layer.
//!
//! Initialization is split into two phases so the kernel can bring up
//! the heap *between* them:
//!
//!   * `init_early` — CPU feature probe + GDT + IDT.  No heap, no MMIO
//!                    mapping; uses static storage only.
//!   * `init_late`  — APIC (needs `map_mmio` → frame_alloc) + enables
//!                    hardware IRQs.
#![allow(unused_imports)]

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64 as current;

/// Phase 1 — runs before `memory::init`.  Does **not** allocate.
pub fn init_early() {
    current::cpu::early_cpu_setup();
    current::gdt::init();
    current::idt::init();
    log::info!("[arch] early init complete");
}

/// Phase 2 — runs after `memory::init`.  Maps MMIO and turns on IRQs.
pub fn init_late() {
    current::apic::init();
    current::interrupts::init(); // sti — must be last.
    log::info!("[arch] late init complete");
}

/// Halt the current CPU until the next interrupt arrives.
#[inline(always)]
pub fn halt() { current::cpu::halt(); }

/// Globally enable hardware interrupts.
#[inline(always)]
pub fn enable_irq() { current::cpu::enable_interrupts(); }

/// Globally disable hardware interrupts.
#[inline(always)]
pub fn disable_irq() { current::cpu::disable_interrupts(); }
