//! Control register (CR0/CR3/CR4) helpers.
//!
//! Stage 2 enables CR4 security bits gated on CPUID:
//!   * **SMEP** — kernel cannot execute user pages.
//!   * **SMAP** — kernel cannot read user pages without `clac`.
//!   * **UMIP** — ring 3 cannot read SGDT/SIDT/SLDT/STR/SMSW.
//!   * **FSGSBASE** — fast user-mode GS swap (per-CPU data).
//!   * **PCIDE** — process-context IDs (TLB tagging).
use core::arch::asm;

pub mod cr0 {
    pub const PE: u64 = 1 << 0;     // protected mode
    pub const MP: u64 = 1 << 1;
    pub const EM: u64 = 1 << 2;
    pub const TS: u64 = 1 << 3;
    pub const WP: u64 = 1 << 16;    // write protect (kernel can't write RO pages)
    pub const PG: u64 = 1 << 31;    // paging enable
}

pub mod cr4 {
    pub const VME:      u64 = 1 << 0;
    pub const DE:       u64 = 1 << 3;
    pub const PSE:      u64 = 1 << 4;
    pub const PAE:      u64 = 1 << 5;
    pub const PGE:      u64 = 1 << 7;
    pub const PCE:      u64 = 1 << 8;
    pub const OSFXSR:   u64 = 1 << 9;
    pub const OSXMMEXCPT: u64 = 1 << 10;
    pub const UMIP:     u64 = 1 << 11;
    pub const FSGSBASE: u64 = 1 << 16;
    pub const PCIDE:    u64 = 1 << 17;
    pub const OSXSAVE:  u64 = 1 << 18;
    pub const SMEP:     u64 = 1 << 20;
    pub const SMAP:     u64 = 1 << 21;
    pub const CET:      u64 = 1 << 23;
}

#[inline]
pub fn read_cr0() -> u64 {
    let v: u64;
    unsafe { asm!("mov {}, cr0", out(reg) v, options(nomem, nostack)) };
    v
}

#[inline]
pub unsafe fn write_cr0(v: u64) {
    asm!("mov cr0, {}", in(reg) v, options(nomem, nostack));
}

#[inline]
pub fn read_cr3() -> u64 {
    let v: u64;
    unsafe { asm!("mov {}, cr3", out(reg) v, options(nomem, nostack)) };
    v
}

#[inline]
pub unsafe fn write_cr3(v: u64) {
    asm!("mov cr3, {}", in(reg) v, options(nomem, nostack));
}

#[inline]
pub fn read_cr4() -> u64 {
    let v: u64;
    unsafe { asm!("mov {}, cr4", out(reg) v, options(nomem, nostack)) };
    v
}

#[inline]
pub unsafe fn write_cr4(v: u64) {
    asm!("mov cr4, {}", in(reg) v, options(nomem, nostack));
}

/// Atomically `OR` a CR4 mask in-place.
#[inline]
pub unsafe fn set_cr4_bits(mask: u64) {
    write_cr4(read_cr4() | mask);
}

/// `clac` — clear AC, suppresses SMAP for explicit user-page access windows.
#[inline]
pub unsafe fn clac() { asm!("clac", options(nomem, nostack)) }

/// `stac` — set AC, allows kernel to access user pages.
#[inline]
pub unsafe fn stac() { asm!("stac", options(nomem, nostack)) }
