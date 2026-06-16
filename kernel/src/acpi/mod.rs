//! ACPI subsystem.
//!
//! Pipeline:
//!   1. `rsdp::find` — locate the Root System Description Pointer
//!      (BIOS scan @ 0xE0000-0xFFFFF or UEFI Configuration Table).
//!   2. `xsdt::parse` (or rsdt fallback) — list every ACPI table.
//!   3. `madt`, `hpet`, `mcfg` — parse the ones we care about.
//!
//! All parsing is **bound-checked** — any malformed table is rejected
//! with a logged warning rather than crashing the kernel.
pub mod hpet;
pub mod madt;
pub mod mcfg;
pub mod rsdp;
pub mod sdt;
pub mod xsdt;

use crate::memory::address::PhysAddr;

/// One-shot ACPI walk.  Returns a snapshot of everything we found.
pub struct AcpiInfo {
    pub madt:  Option<madt::Madt>,
    pub hpet:  Option<hpet::Hpet>,
    pub mcfg:  Option<mcfg::Mcfg>,
}

pub fn parse() -> AcpiInfo {
    let info = (|| -> Option<AcpiInfo> {
        let rsdp = rsdp::find()?;
        let xsdt_phys = PhysAddr::new(rsdp.xsdt_addr() as usize);
        let entries = xsdt::list_entries(xsdt_phys)?;

        let mut madt = None;
        let mut hpet = None;
        let mut mcfg = None;

        for &phys in &entries {
            if let Some(sig) = sdt::signature_at(phys) {
                match &sig {
                    b"APIC" => madt = madt::parse(phys),
                    b"HPET" => hpet = hpet::parse(phys),
                    b"MCFG" => mcfg = mcfg::parse(phys),
                    _ => {}
                }
            }
        }
        Some(AcpiInfo { madt, hpet, mcfg })
    })();

    info.unwrap_or_else(|| {
        log::warn!("[acpi] not available — fallback mode");
        AcpiInfo { madt: None, hpet: None, mcfg: None }
    })
}
