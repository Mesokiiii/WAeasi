//! MADT — Multiple APIC Description Table.
//!
//! Tells us:
//!   * Local APIC address (and overrides).
//!   * The list of CPUs (one entry per processor).
//!   * The list of I/O APICs.
//!   * Interrupt source overrides (mostly for legacy IRQ remapping).
use alloc::vec::Vec;

use crate::memory::address::PhysAddr;

use super::sdt::{slice_at, HEADER_LEN};

#[derive(Debug, Clone)]
pub struct Madt {
    pub local_apic_phys: u32,
    pub cpus:            Vec<crate::arch::x86_64::smp::topology::Cpu>,
    pub ioapics:         Vec<IoApic>,
    pub overrides:       Vec<IntOverride>,
}

#[derive(Copy, Clone, Debug)]
pub struct IoApic {
    pub id:           u8,
    pub address:      u32,
    pub gsi_base:     u32,
}

#[derive(Copy, Clone, Debug)]
pub struct IntOverride {
    pub bus:    u8,
    pub source: u8,
    pub gsi:    u32,
    pub flags:  u16,
}

const ENTRY_LAPIC:    u8 = 0;
const ENTRY_IOAPIC:   u8 = 1;
const ENTRY_OVERRIDE: u8 = 2;
const ENTRY_X2APIC:   u8 = 9;

pub fn parse(phys: PhysAddr) -> Option<Madt> {
    let bytes = slice_at(phys)?;
    if &bytes[..4] != b"APIC" { return None; }

    let body = &bytes[HEADER_LEN..];
    if body.len() < 8 { return None; }
    let local_apic_phys = u32::from_le_bytes(body[..4].try_into().unwrap());
    let _flags          = u32::from_le_bytes(body[4..8].try_into().unwrap());

    let mut cpus     = Vec::new();
    let mut ioapics  = Vec::new();
    let mut overrides= Vec::new();
    let mut p = 8;

    while p + 2 <= body.len() {
        let kind = body[p];
        let len  = body[p + 1] as usize;
        if len < 2 || p + len > body.len() { break; }
        match kind {
            ENTRY_LAPIC if len >= 8 => {
                cpus.push(crate::arch::x86_64::smp::topology::Cpu {
                    acpi_id: body[p + 2],
                    apic_id: body[p + 3],
                    bsp:     cpus.is_empty(),
                    enabled: u32::from_le_bytes(body[p+4..p+8].try_into().unwrap()) & 1 != 0,
                });
            }
            ENTRY_IOAPIC if len >= 12 => {
                ioapics.push(IoApic {
                    id:       body[p + 2],
                    address:  u32::from_le_bytes(body[p+4..p+8].try_into().unwrap()),
                    gsi_base: u32::from_le_bytes(body[p+8..p+12].try_into().unwrap()),
                });
            }
            ENTRY_OVERRIDE if len >= 10 => {
                overrides.push(IntOverride {
                    bus:    body[p + 2],
                    source: body[p + 3],
                    gsi:    u32::from_le_bytes(body[p+4..p+8].try_into().unwrap()),
                    flags:  u16::from_le_bytes(body[p+8..p+10].try_into().unwrap()),
                });
            }
            ENTRY_X2APIC if len >= 16 => {
                cpus.push(crate::arch::x86_64::smp::topology::Cpu {
                    apic_id: (u32::from_le_bytes(body[p+4..p+8].try_into().unwrap()) & 0xFF) as u8,
                    acpi_id: (u32::from_le_bytes(body[p+12..p+16].try_into().unwrap()) & 0xFF) as u8,
                    bsp:     cpus.is_empty(),
                    enabled: u32::from_le_bytes(body[p+8..p+12].try_into().unwrap()) & 1 != 0,
                });
            }
            _ => {}
        }
        p += len;
    }

    log::info!("[acpi] MADT: {} CPUs, {} I/O APICs, {} overrides",
               cpus.len(), ioapics.len(), overrides.len());
    Some(Madt { local_apic_phys, cpus, ioapics, overrides })
}
