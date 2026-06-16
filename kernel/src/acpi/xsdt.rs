//! XSDT — eXtended System Description Table.
//!
//! `XSDT` body is an array of 8-byte physical pointers to other ACPI
//! tables (MADT, HPET, MCFG, FADT, SSDT, ...).  We list them all so the
//! caller can `signature_at(p)` to discover what's there.
use alloc::vec::Vec;

use crate::memory::address::PhysAddr;

use super::sdt::{slice_at, HEADER_LEN};

pub fn list_entries(xsdt_phys: PhysAddr) -> Option<Vec<PhysAddr>> {
    let bytes = slice_at(xsdt_phys)?;
    if &bytes[..4] != b"XSDT" { return None; }

    let body = &bytes[HEADER_LEN..];
    let n_entries = body.len() / 8;
    let mut out = Vec::with_capacity(n_entries);
    for i in 0..n_entries {
        let off = i * 8;
        let raw = u64::from_le_bytes(body[off..off + 8].try_into().unwrap());
        out.push(PhysAddr::new(raw as usize));
    }
    log::info!("[acpi] XSDT lists {} tables", out.len());
    Some(out)
}
