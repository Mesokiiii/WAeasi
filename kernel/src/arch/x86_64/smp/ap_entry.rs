//! Application-Processor entry — first 64-bit Rust call on each AP.
//!
//! Per Intel SDM § 8.4: an AP wakes in 16-bit real mode, the trampoline
//! enables protected → long mode, then jumps here.  We inherit the BSP's
//! page tables, so memory is already valid.
//!
//! Each AP must:
//!   1. Load the GDT/IDT (shared with the BSP — same descriptors).
//!   2. Set its `IA32_GS_BASE` to its per-CPU slot.
//!   3. Bring its LAPIC online + enable interrupts.
//!   4. Increment `ONLINE_CPUS` so the BSP can release the wait.
//!   5. Run the per-CPU executor forever.
use core::sync::atomic::Ordering;

use super::ONLINE_CPUS;
use crate::arch::x86_64::{apic, gdt, idt, per_cpu};
use crate::sched::executor::Executor;

/// First-class AP entry — symbol referenced by the trampoline blob.
#[no_mangle]
pub extern "C" fn ap_main(cpu_id: u32) -> ! {
    // 1. Load the shared descriptor tables on this CPU.
    gdt::init();
    idt::init();

    // 2. Bind GS_BASE to this AP's per-CPU slot.
    unsafe { bind_per_cpu(cpu_id); }

    // 3. Enable LAPIC + IRQs.
    apic::init();
    crate::arch::enable_irq();

    // 4. Announce to the BSP.
    ONLINE_CPUS.fetch_add(1, Ordering::AcqRel);
    log::info!("[smp] CPU {} online", cpu_id);

    // 5. Run forever.
    Executor::for_cpu(cpu_id).run()
}

unsafe fn bind_per_cpu(cpu_id: u32) {
    use crate::arch::x86_64::msr;
    let slot = per_cpu::slot_address(cpu_id as usize);
    msr::wrmsr(msr::id::IA32_GS_BASE, slot as u64);
}
