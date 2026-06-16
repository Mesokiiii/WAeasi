//! Per-CPU data via `gs:0`.
//!
//! On x86_64 we point `IA32_GS_BASE` at a per-CPU `PerCpu` struct.  That
//! lets the kernel access the current CPU's executor / canary / IRQ-stack
//! using a single `mov rax, gs:[offset]` — branchless, cache-friendly,
//! SMP-correct.
//!
//! Stage 2 has a single CPU; the API is already SMP-shaped (vector of
//! per-CPU slots, indexed by APIC id).
use alloc::vec::Vec;

use super::msr;

/// Per-CPU data block.  Aligned to a cache line so neighbouring CPUs
/// don't fight over its line.
#[repr(C, align(64))]
pub struct PerCpu {
    /// 0:  Self pointer — gs:[0] returns the address of this struct.
    pub self_ptr:        usize,
    /// 8:  CPU identifier (0..N_CPU).
    pub cpu_id:          u32,
    /// 12: Stack-canary value used by `security::canary`.
    pub stack_canary:    u64,
    /// 20: Pointer to the IRQ stack top.
    pub irq_stack_top:   usize,
    /// 28: Reentry counter — incremented on every IRQ/exception entry,
    ///     decremented on exit.  0 in normal task context.
    pub irq_depth:       u32,
}

static mut SLOTS: Vec<PerCpu> = Vec::new();

/// Initialize the per-CPU vector for `cpu_count` CPUs and write the
/// current CPU's `gs:base` to point at slot 0.
pub fn init(cpu_count: usize) {
    unsafe {
        SLOTS = Vec::with_capacity(cpu_count);
        for id in 0..cpu_count {
            SLOTS.push(PerCpu {
                self_ptr:      0,
                cpu_id:        id as u32,
                stack_canary:  0,
                irq_stack_top: 0,
                irq_depth:     0,
            });
            // Patch self_ptr after the Vec settles — Vec may have
            // reallocated during pushes, so we fix it up below in a
            // second pass.
        }
        for id in 0..cpu_count {
            let p = &mut SLOTS[id] as *mut PerCpu as usize;
            SLOTS[id].self_ptr = p;
        }
        // Set GS_BASE for the current CPU (slot 0).
        let base = SLOTS.as_ptr() as u64;
        msr::wrmsr(msr::id::IA32_GS_BASE, base);
    }
    log::info!("[per_cpu] {} slot(s) initialized", cpu_count);
}

/// Read the current CPU's `PerCpu` reference via `gs:0`.
#[inline(always)]
pub fn current() -> &'static mut PerCpu {
    let p: usize;
    unsafe { core::arch::asm!("mov {}, gs:0", out(reg) p, options(nostack, preserves_flags)) };
    unsafe { &mut *(p as *mut PerCpu) }
}

/// Convenience: current CPU id.
#[inline(always)]
pub fn cpu_id() -> u32 { current().cpu_id }

/// Address of the per-CPU slot for `cpu` — used by SMP AP startup.
pub fn slot_address(cpu: usize) -> usize {
    unsafe { &SLOTS[cpu] as *const PerCpu as usize }
}
