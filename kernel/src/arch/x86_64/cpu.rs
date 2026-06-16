//! Low-level CPU helpers: feature probing, halting, IRQ flags, RDTSC.
//!
//! IRQ-flag manipulation is intentionally done with **fused** `pushfq; cli`
//! sequences so an interrupt cannot fire between observation and masking.
use core::arch::asm;

/// Saved IRQ-flag state — opaque, only consumed by `restore_interrupts`.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct IrqState(u64);

impl IrqState {
    #[inline(always)]
    pub fn was_enabled(self) -> bool { self.0 & (1 << 9) != 0 }
}

/// Called at the very start of `arch::init` — reserved for later CPUID
/// feature probing (SSE, AVX, RDRAND, ...).
pub fn early_cpu_setup() {
    // Future: read CPUID, populate a global `CpuFeatures` struct.
}

/// `hlt` — wait until next interrupt. Used by the idle loop in the executor.
#[inline(always)]
pub fn halt() {
    unsafe { asm!("hlt", options(nomem, nostack, preserves_flags)) };
}

/// Globally enable maskable interrupts (`sti`).
#[inline(always)]
pub fn enable_interrupts() {
    unsafe { asm!("sti", options(nomem, nostack)) };
}

/// Globally disable maskable interrupts (`cli`).
#[inline(always)]
pub fn disable_interrupts() {
    unsafe { asm!("cli", options(nomem, nostack)) };
}

/// Atomically read RFLAGS *and* mask interrupts.  Returns the previous
/// state so the caller can restore it via `restore_interrupts`.
#[inline(always)]
pub fn save_and_disable_interrupts() -> IrqState {
    let r: u64;
    unsafe {
        asm!(
            "pushfq",
            "pop {0}",
            "cli",
            out(reg) r,
            options(nomem),
        );
    }
    IrqState(r)
}

/// Restore IF to whatever it was when `save_and_disable_interrupts` ran.
#[inline(always)]
pub fn restore_interrupts(state: IrqState) {
    if state.was_enabled() {
        unsafe { asm!("sti", options(nomem, nostack)) };
    }
}

/// Read the timestamp counter (cycles since reset).
#[inline(always)]
pub fn rdtsc() -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe { asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack)) };
    ((hi as u64) << 32) | (lo as u64)
}

/// Run `f` with interrupts disabled, restoring the previous IF flag on exit.
#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let s = save_and_disable_interrupts();
    let r = f();
    restore_interrupts(s);
    r
}

/// Spin-wait that yields to the CPU — for short backoff in busy loops.
#[inline(always)]
pub fn pause() {
    unsafe { asm!("pause", options(nomem, nostack, preserves_flags)) };
}
