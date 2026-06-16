//! AP startup trampoline.
//!
//! AP wakeup sequence:
//!   1. Copy a 16-bit real-mode bootstrap to a low (<1 MiB) page —
//!      stage 3 reserves page `0x8000` (vector 0x08).
//!   2. INIT IPI to target APIC ID, wait 10 ms.
//!   3. SIPI IPI carrying the trampoline page index, wait 200 µs.
//!   4. Second SIPI (recommended by Intel SDM § 8.4.4).
//!   5. AP bootstraps real → protected → long mode and jumps to
//!      `ap_entry::main`.
//!
//! The trampoline machine-code blob is provided by stage 4 — until then
//! `wakeup()` returns `Err("trampoline blob missing")` so the boot path
//! falls back to single-CPU.
use core::sync::atomic::{AtomicBool, Ordering};

const TRAMPOLINE_VECTOR: u8 = 0x08;          // physical page 0x8000
const TRAMPOLINE_PAGE:   usize = 0x8000;

static INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn install() {
    if INSTALLED.swap(true, Ordering::AcqRel) { return; }
    log::debug!("[smp] trampoline reserved @ {:#x}", TRAMPOLINE_PAGE);
    // Stage 4: copy the assembled blob bytes here using the direct map.
}

/// Send INIT-SIPI-SIPI to the AP with `apic_id`.  Returns the boot
/// status.  Stage 3 logs the intent and reports unavailable; the actual
/// LAPIC ICR poke ships when ACPI is wired in.
pub fn wakeup(apic_id: u8) -> Result<(), &'static str> {
    if !INSTALLED.load(Ordering::Acquire) {
        return Err("trampoline not installed");
    }
    log::info!("[smp] wakeup AP {} (vector {:#x})", apic_id, TRAMPOLINE_VECTOR);
    Err("trampoline blob missing — stage 4")
}

/// Programmatic IPI helper — exposed for future RTC-clock /
/// scheduler-IPI use.
pub unsafe fn send_ipi(_apic_id: u8, _vector: u8, _delivery: IpiKind) {
    // Stage 4 will write to the LAPIC ICR (Interrupt Command Register).
}

#[derive(Copy, Clone, Debug)]
pub enum IpiKind { Fixed, Init, Sipi }
