//! W⊕X (Write XOR Execute) enforcement.
//!
//! Three layers, depth-in-defense:
//!
//!   1. **Page table flags** — code pages have `NO_EXECUTE` cleared,
//!      `WRITABLE` cleared.  Data pages have `NO_EXECUTE` set.  MMIO
//!      windows are NX + uncached (handled in `paging::map_mmio`).
//!
//!   2. **CR0.WP** — even ring 0 cannot write read-only pages.  This
//!      catches `null + offset` writes through stale pointers.
//!
//!   3. **CR4.SMEP** — kernel cannot execute user-mapped code.  Even if
//!      a buggy host function jumps to a user address, the CPU faults.
//!
//!   4. **CR4.SMAP** — kernel cannot read user-mapped data without an
//!      explicit `stac` bracket.  Defeats null-deref-style probes.
//!
//!   5. **EFER.NXE** — required for the NX bit to take effect.
use crate::arch::x86_64::{cpuid::CpuFeatures, cr, msr};

/// Enable every W⊕X-related CPU feature that this CPU advertises.
pub fn enable(f: &CpuFeatures) {
    unsafe {
        // EFER.NXE — must be set before any PT entry uses bit 63.
        if f.nx {
            msr::set_msr_bits(msr::id::IA32_EFER, msr::efer::NXE);
        }

        // CR0.WP — write-protect even for ring 0.
        let mut c0 = cr::read_cr0();
        c0 |= cr::cr0::WP;
        cr::write_cr0(c0);

        // CR4 bits gated on CPUID.
        let mut mask = 0u64;
        if f.smep     { mask |= cr::cr4::SMEP; }
        if f.smap     { mask |= cr::cr4::SMAP; }
        if f.umip     { mask |= cr::cr4::UMIP; }
        if f.fsgsbase { mask |= cr::cr4::FSGSBASE; }
        if f.pcid     { mask |= cr::cr4::PCIDE; }
        if mask != 0 { cr::set_cr4_bits(mask); }
    }
    log::info!("[wxn] enabled: NX={} WP=1 SMEP={} SMAP={} UMIP={}",
               f.nx, f.smep, f.smap, f.umip);
}

/// RAII guard for the rare case kernel must touch user memory: opens a
/// SMAP window via `stac`, closes it via `clac` on drop.
pub struct UserAccessWindow;

impl UserAccessWindow {
    /// SAFETY: caller asserts the access target is well-formed and
    /// transient.  Misuse re-introduces SMAP-violating reads.
    pub unsafe fn open() -> Self { cr::stac(); Self }
}

impl Drop for UserAccessWindow {
    fn drop(&mut self) { unsafe { cr::clac() } }
}
