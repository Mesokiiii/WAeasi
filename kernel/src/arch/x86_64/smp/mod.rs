//! Symmetric multiprocessing — bring secondary CPUs online.
//!
//! Pipeline:
//!   1. `topology::probe`  — read MADT (Multiple APIC Description Table)
//!      from ACPI to enumerate every Local APIC.
//!   2. `trampoline::install` — copy a 16-bit → 64-bit trampoline blob
//!      to a low (<1 MiB) physical page so APs can start in real mode.
//!   3. `start_aps`        — issue INIT-SIPI-SIPI to each AP; wait for
//!      `online` flag.
//!   4. `ap_entry::main`   — first 64-bit Rust call on the AP — sets up
//!      GDT/IDT/per-CPU GS, then calls `executor::run` for that CPU.
pub mod ap_entry;
pub mod topology;
pub mod trampoline;

use core::sync::atomic::{AtomicU32, Ordering};

pub static ONLINE_CPUS: AtomicU32 = AtomicU32::new(1); // BSP is online

/// Bring up every detected AP.  Returns the count actually online.
pub fn start_aps() -> u32 {
    let cpus = topology::probe();
    if cpus.len() <= 1 {
        log::info!("[smp] single CPU, skipping AP startup");
        return 1;
    }
    trampoline::install();

    for c in cpus.iter().skip(1) {
        if let Err(e) = trampoline::wakeup(c.apic_id) {
            log::warn!("[smp] CPU {} failed to wake: {}", c.apic_id, e);
        }
    }

    // Wait up to 100 ms for APs to bump ONLINE_CPUS.
    let deadline = crate::arch::x86_64::interrupts::ticks() + 10;
    while crate::arch::x86_64::interrupts::ticks() < deadline {
        if ONLINE_CPUS.load(Ordering::Acquire) as usize == cpus.len() { break; }
        crate::arch::halt();
    }

    let n = ONLINE_CPUS.load(Ordering::Acquire);
    log::info!("[smp] {} / {} CPUs online", n, cpus.len());
    n
}
