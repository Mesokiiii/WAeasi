//! MCFG — PCI Express Memory-mapped Configuration Space.
//!
//! Tells us the base of the PCIe ECAM region, which lets us read PCI
//! config space via a normal memory load instead of legacy 0xCF8/0xCFC.
use alloc::vec::Vec;

use crate::memory::address::PhysAddr;

use super::sdt::{slice_at, HEADER_LEN};

#[derive(Copy, Clone, Debug)]
pub struct Allocation {
    pub base:        u64,
    pub pci_segment: u16,
    pub start_bus:   u8,
    pub end_bus:     u8,
}

#[derive(Debug, Clone)]
pub struct Mcfg {
    pub allocations: Vec<Allocation>,
}

pub fn parse(phys: PhysAddr) -> Option<Mcfg> {
    let bytes = slice_at(phys)?;
    if &bytes[..4] != b"MCFG" { return None; }

    let body = &bytes[HEADER_LEN..];
    if body.len() < 8 { return None; }

    let mut allocations = Vec::new();
    let mut p = 8; // 8-byte reserved
    while p + 16 <= body.len() {
        allocations.push(Allocation {
            base:        u64::from_le_bytes(body[p..p+8].try_into().unwrap()),
            pci_segment: u16::from_le_bytes(body[p+8..p+10].try_into().unwrap()),
            start_bus:   body[p + 10],
            end_bus:     body[p + 11],
        });
        p += 16;
    }
    log::info!("[acpi] MCFG: {} ECAM allocation(s)", allocations.len());
    Some(Mcfg { allocations })
}
