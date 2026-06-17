//! Exception decoding helpers.
//!
//! Concrete `extern "x86-interrupt"` handlers live in `idt.rs` —
//! intentionally minimal so a fault inside the formatting machinery
//! cannot recurse.  This module is kept as the public type surface
//! for callers that inspect a `#PF` error code (currently the demand-
//! paging path in `memory::demand`).

bitflags::bitflags! {
    /// Decoded `error_code` bits from a `#PF` exception.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PfFlags: u64 {
        const PRESENT          = 1 << 0;
        const WRITE            = 1 << 1;
        const USER             = 1 << 2;
        const RESERVED_WRITE   = 1 << 3;
        const INSTRUCTION_FETCH = 1 << 4;
        const PROTECTION_KEY   = 1 << 5;
        const SHADOW_STACK     = 1 << 6;
        const SGX              = 1 << 15;
    }
}
