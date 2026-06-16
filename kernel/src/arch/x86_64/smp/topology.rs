//! ACPI MADT (Multiple APIC Description Table) parser.
//!
//! Walks the MADT to collect Local APIC entries — each one describes one
//! CPU.  For stage 3 we accept the BSP only when ACPI is unavailable
//! (single-CPU fallback).
use alloc::vec::Vec;

#[derive(Copy, Clone, Debug)]
pub struct Cpu {
    pub apic_id:  u8,
    pub acpi_id:  u8,
    pub bsp:      bool,
    pub enabled:  bool,
}

/// MADT entry types we care about.
const MADT_LAPIC:  u8 = 0;
const MADT_X2APIC: u8 = 9;

/// Probe topology.  Returns the BSP first, APs after.
///
/// Stage 3 stub: ACPI table walking is the next-stage delivery; until
/// then we report a single CPU (the BSP), so the rest of the kernel
/// builds correctly on a single core.
pub fn probe() -> Vec<Cpu> {
    let mut out = Vec::new();
    out.push(Cpu { apic_id: 0, acpi_id: 0, bsp: true, enabled: true });
    log::debug!("[topology] {} CPU(s) detected (ACPI walk pending)", out.len());
    out
}

/// Parse a MADT blob into `Vec<Cpu>`.  Exposed so a future ACPI parser
/// can feed us the table contents directly.
pub fn parse_madt(blob: &[u8]) -> Vec<Cpu> {
    let mut out = Vec::new();
    if blob.len() < 44 { return out; }
    let mut p = 44;
    while p + 2 <= blob.len() {
        let kind = blob[p];
        let len  = blob[p + 1] as usize;
        if len < 2 || p + len > blob.len() { break; }
        match kind {
            MADT_LAPIC if len >= 8 => {
                let acpi_id = blob[p + 2];
                let apic_id = blob[p + 3];
                let flags   = u32::from_le_bytes([blob[p+4], blob[p+5], blob[p+6], blob[p+7]]);
                out.push(Cpu {
                    apic_id, acpi_id,
                    bsp: out.is_empty(),
                    enabled: flags & 1 != 0,
                });
            }
            MADT_X2APIC if len >= 16 => {
                let apic_id_u32 = u32::from_le_bytes([blob[p+4], blob[p+5], blob[p+6], blob[p+7]]);
                let flags       = u32::from_le_bytes([blob[p+8], blob[p+9], blob[p+10], blob[p+11]]);
                let acpi_id     = u32::from_le_bytes([blob[p+12], blob[p+13], blob[p+14], blob[p+15]]);
                out.push(Cpu {
                    apic_id: (apic_id_u32 & 0xFF) as u8,
                    acpi_id: (acpi_id & 0xFF) as u8,
                    bsp: out.is_empty(),
                    enabled: flags & 1 != 0,
                });
            }
            _ => {}
        }
        p += len;
    }
    out
}
