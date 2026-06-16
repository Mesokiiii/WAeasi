//! Model-Specific Register access.
//!
//! `rdmsr` / `wrmsr` are **privileged** (CPL 0 only).  All callers are
//! kernel code; we still mark every helper `unsafe` because writing the
//! wrong value to many MSRs (EFER, IA32_PAT, ...) crashes the box.
use core::arch::asm;

/// Numeric MSR identifiers we touch in stage 2.
pub mod id {
    pub const IA32_EFER:               u32 = 0xC000_0080;
    pub const IA32_FS_BASE:            u32 = 0xC000_0100;
    pub const IA32_GS_BASE:            u32 = 0xC000_0101;
    pub const IA32_KERNEL_GS_BASE:     u32 = 0xC000_0102;
    pub const IA32_PAT:                u32 = 0x0000_0277;
    pub const IA32_SPEC_CTRL:          u32 = 0x0000_0048;
    pub const IA32_PRED_CMD:           u32 = 0x0000_0049;
    pub const IA32_FLUSH_CMD:          u32 = 0x0000_010B;
    pub const IA32_TSC_DEADLINE:       u32 = 0x0000_06E0;
    pub const IA32_APIC_BASE:          u32 = 0x0000_001B;
}

/// EFER bits.
pub mod efer {
    pub const SCE: u64 = 1 << 0;     // syscall enable
    pub const LME: u64 = 1 << 8;     // long mode enable
    pub const LMA: u64 = 1 << 10;    // long mode active
    pub const NXE: u64 = 1 << 11;    // no-execute enable
}

/// SPEC_CTRL bits (Intel SDM Vol 4 — Speculation control).
pub mod spec_ctrl {
    pub const IBRS: u64 = 1 << 0;
    pub const STIBP: u64 = 1 << 1;
    pub const SSBD: u64 = 1 << 2;
}

/// PRED_CMD bits.
pub mod pred_cmd {
    pub const IBPB: u64 = 1 << 0; // indirect branch predictor barrier
}

/// Read a 64-bit MSR.
#[inline]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let lo: u32; let hi: u32;
    asm!("rdmsr", in("ecx") msr, out("eax") lo, out("edx") hi, options(nomem, nostack));
    ((hi as u64) << 32) | (lo as u64)
}

/// Write a 64-bit MSR.
#[inline]
pub unsafe fn wrmsr(msr: u32, val: u64) {
    let lo = val as u32;
    let hi = (val >> 32) as u32;
    asm!("wrmsr", in("ecx") msr, in("eax") lo, in("edx") hi, options(nomem, nostack));
}

/// Atomically `OR` a bit into an MSR — useful for enabling features.
#[inline]
pub unsafe fn set_msr_bits(msr: u32, mask: u64) {
    let v = rdmsr(msr);
    wrmsr(msr, v | mask);
}

/// Atomically `AND` against an MSR — disables bits.
#[inline]
pub unsafe fn clear_msr_bits(msr: u32, mask: u64) {
    let v = rdmsr(msr);
    wrmsr(msr, v & !mask);
}
