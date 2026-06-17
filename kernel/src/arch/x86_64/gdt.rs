//! Global Descriptor Table.
//!
//! Diagnostic minimal version: the boot32 trampoline already installed
//! a working long-mode GDT (`0x08` = code, `0x10` = data), so for the
//! purposes of "make IDT work and let exceptions surface" we can keep
//! using it as-is.  Once the rest of the boot pipeline is alive we
//! will reinstate a TSS with a dedicated `#DF` IST stack.
use core::sync::atomic::{AtomicBool, Ordering};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn init() {
    if INITIALIZED.swap(true, Ordering::AcqRel) {
        return;
    }
    log::debug!("[gdt] using boot32-supplied GDT (CS=0x08, DS=0x10)");
}
