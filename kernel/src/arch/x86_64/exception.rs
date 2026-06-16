//! Exception dispatch — separated from `idt.rs` so the IDT module stays
//! a pure registration table.  Stage 3 makes `#PF` handle demand paging
//! for Wasm linear memories.
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};

use crate::memory::address::VirtAddr;

bitflags::bitflags! {
    /// Decoded `error_code` bits from a #PF exception.
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

/// Top-level #PF handler — dispatches to demand-paging or panics.
pub extern "x86-interrupt" fn page_fault(stack: InterruptStackFrame, ec: PageFaultErrorCode) {
    let cr2 = Cr2::read().map(|a| a.as_u64()).unwrap_or(0);
    let flags = PfFlags::from_bits_truncate(ec.bits());

    // 1. Demand-paging path for the Wasm linear-memory arena.
    if let Some(_) = crate::memory::demand::try_handle(VirtAddr::new(cr2 as usize), flags) {
        return;
    }

    // 2. Real kernel fault — print full diagnostics, then halt.
    cold_panic(cr2, flags, &stack);
}

#[cold]
fn cold_panic(cr2: u64, flags: PfFlags, frame: &InterruptStackFrame) -> ! {
    panic!(
        "#PF cr2={:#018x} flags={:?} ip={:?} cs={} rsp={:?}",
        cr2, flags, frame.instruction_pointer, frame.code_segment.0, frame.stack_pointer
    );
}

/// Generic protection-fault diagnostic.
#[cold]
pub fn report_fault(name: &str, frame: &InterruptStackFrame, ec: u64) -> ! {
    panic!(
        "{} ec={:#x} ip={:?} cs={}",
        name, ec, frame.instruction_pointer, frame.code_segment.0
    );
}
