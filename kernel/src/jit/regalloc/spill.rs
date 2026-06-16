//! Spill-slot management.
//!
//! Each spilled live-range gets a fixed offset from `rbp` in the
//! function's local-stack frame.  Slots are 8 bytes wide so a Wasm
//! `Cell` (= 8-byte raw value) fits in one slot without packing.
use alloc::vec::Vec;

use super::scan::RangeId;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpillSlot(pub u32);

pub struct SpillManager {
    /// `slots[i]` records which `RangeId` owns slot `i`, if any.
    slots: Vec<Option<RangeId>>,
}

impl SpillManager {
    pub fn new() -> Self { Self { slots: Vec::new() } }

    /// Reserve a slot for `range`.  Returns the slot index.
    pub fn alloc(&mut self, range: RangeId) -> SpillSlot {
        for (i, s) in self.slots.iter_mut().enumerate() {
            if s.is_none() { *s = Some(range); return SpillSlot(i as u32); }
        }
        self.slots.push(Some(range));
        SpillSlot((self.slots.len() - 1) as u32)
    }

    /// Free the slot owned by `range`.
    pub fn free(&mut self, range: RangeId) {
        for s in self.slots.iter_mut() {
            if matches!(*s, Some(r) if r == range) { *s = None; return; }
        }
    }

    pub fn frame_bytes(&self) -> u32 { (self.slots.len() as u32) * 8 }

    /// Pick a victim live-range to spill: the latest-ending active range.
    pub fn pick_victim(active: &[(RangeId, u32)]) -> Option<RangeId> {
        active.iter().max_by_key(|(_, end)| *end).map(|(id, _)| *id)
    }
}
