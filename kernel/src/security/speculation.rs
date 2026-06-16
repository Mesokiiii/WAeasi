//! Speculative-execution attack mitigations.
//!
//! Coverage:
//!   * **Spectre v1** (Bounds-check bypass) — `array_index_nospec` clamps
//!     attacker-controlled indices via `cmov` + `lfence`.  No branch ever
//!     speculates past a bounds check.
//!   * **Spectre v2** (Branch target injection) — IBRS / STIBP / IBPB
//!     (when CPU + uCode supports them) flush the indirect-branch predictor
//!     on transitions to / from less-trusted contexts.
//!   * **Spectre v4** (Speculative store bypass) — SSBD bit prevents store
//!     forwarding from speculatively guessed values.
//!
//! Any of these MSRs missing degrades gracefully — we just log.
use core::arch::asm;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86_64::{cpuid::CpuFeatures, msr};

static SPEC_V2_HARD: AtomicBool = AtomicBool::new(false);

/// Turn on every available speculation mitigation.
pub fn enable(f: &CpuFeatures) {
    let mut spec_ctrl = 0u64;
    if f.ibrs_ibpb { spec_ctrl |= msr::spec_ctrl::IBRS; }
    if f.stibp     { spec_ctrl |= msr::spec_ctrl::STIBP; }
    if f.ssbd      { spec_ctrl |= msr::spec_ctrl::SSBD; }

    if spec_ctrl != 0 {
        unsafe { msr::set_msr_bits(msr::id::IA32_SPEC_CTRL, spec_ctrl) };
        SPEC_V2_HARD.store(f.ibrs_ibpb, Ordering::Release);
    }
    log::info!("[speculation] IBRS={} STIBP={} SSBD={} v1_lfence=enabled",
               f.ibrs_ibpb, f.stibp, f.ssbd);
}

/// Spectre v1 protection: clamp `index` to `< len` *speculatively safely*.
///
/// Standard `if i < len { a[i] }` is safe architecturally but a
/// mispredicted branch can speculate `a[i]` for `i >= len` and leak
/// the byte through cache.  This helper:
///
///   1. Computes a saturating mask via `cmov` (no branch).
///   2. Issues `lfence` so the load below cannot be reordered ahead.
///
/// Use it in any host call that derives an offset from Wasm-supplied data.
#[inline(always)]
pub fn array_index_nospec(index: usize, len: usize) -> usize {
    let mut clamped = index;
    unsafe {
        // if !(index < len) { clamped = 0 }  (branch-free)
        asm!(
            "cmp {idx}, {len}",
            "cmovae {clamped}, {zero}",
            idx     = in(reg) index,
            len     = in(reg) len,
            clamped = inout(reg) clamped,
            zero    = in(reg) 0usize,
            options(pure, nomem, nostack),
        );
        asm!("lfence", options(nomem, nostack));
    }
    clamped
}

/// `lfence` — serialize execution; nothing past this point can have
/// already speculated.
#[inline(always)]
pub fn barrier() {
    unsafe { asm!("lfence", options(nomem, nostack, preserves_flags)) };
}

/// Issue an IBPB (Indirect Branch Predictor Barrier) — flush the BTB.
/// Call before/after switching from a Wasm component back to kernel code.
#[inline]
pub fn ibpb() {
    if SPEC_V2_HARD.load(Ordering::Relaxed) {
        unsafe { msr::wrmsr(msr::id::IA32_PRED_CMD, msr::pred_cmd::IBPB) };
    }
}
