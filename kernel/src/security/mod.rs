//! Kernel security subsystem.
//!
//! WAeasi's threat model assumes Wasm components are **untrusted**.  The
//! kernel itself is trusted but enforces depth-in-defense:
//!
//!   * **Capability tokens** (`capability`) — every privileged operation
//!     requires an unforgeable token that names exactly one right.
//!   * **Audit trail** (`audit`) — every grant/revoke/check is logged in
//!     a ring buffer for post-mortem analysis.
//!   * **W⊕X** (`wxn`) — code pages are NX-clear + RW-clear, data pages
//!     are NX-set.  Enforced by paging flags + CR4.SMEP.
//!   * **Speculation** (`speculation`) — Spectre v1 (lfence around
//!     attacker-controlled indices) + v2 (IBRS / STIBP / IBPB) +
//!     v4 (SSBD) where CPU supports it.
//!   * **Stack canaries** (`canary`) — random per-CPU value placed
//!     between local frames + return addresses.
//!
//! All of this is initialized **before** the first Wasm component runs.
pub mod audit;
pub mod canary;
pub mod capability;
pub mod speculation;
pub mod wxn;

/// Enable all hardware mitigations + warm caches.  Idempotent.
pub fn init() {
    let f = crate::arch::x86_64::cpuid::probe();
    wxn::enable(f);
    speculation::enable(f);
    canary::init();
    audit::init();
    capability::init();
    log::info!("[security] subsystem initialized (NX={} SMEP={} SMAP={} IBRS={})",
               f.nx, f.smep, f.smap, f.ibrs_ibpb);
}
