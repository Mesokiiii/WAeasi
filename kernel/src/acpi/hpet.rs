//! HPET ACPI table — points us at the HPET MMIO base for time
//! calibration.
use crate::memory::address::PhysAddr;

use super::sdt::{slice_at, HEADER_LEN};

#[derive(Debug, Clone, Copy)]
pub struct Hpet {
    pub event_timer_block_id: u32,
    pub address_space:        u8,
    pub register_bit_width:   u8,
    pub register_bit_offset:  u8,
    pub access_size:          u8,
    pub address:              u64,
    pub hpet_number:          u8,
    pub min_clock_tick:       u16,
}

pub fn parse(phys: PhysAddr) -> Option<Hpet> {
    let bytes = slice_at(phys)?;
    if &bytes[..4] != b"HPET" { return None; }

    let body = &bytes[HEADER_LEN..];
    if body.len() < 24 { return None; }

    let h = Hpet {
        event_timer_block_id: u32::from_le_bytes(body[0..4].try_into().unwrap()),
        address_space:        body[4],
        register_bit_width:   body[5],
        register_bit_offset:  body[6],
        access_size:          body[7],
        address:              u64::from_le_bytes(body[8..16].try_into().unwrap()),
        hpet_number:          body[16],
        min_clock_tick:       u16::from_le_bytes(body[17..19].try_into().unwrap()),
    };
    log::info!("[acpi] HPET @ {:#x}", h.address);
    Some(h)
}
