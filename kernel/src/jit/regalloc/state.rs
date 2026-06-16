//! Allocator state — the live phys-register-pool plus active live ranges.
//!
//! Used by the JIT lowering pass:
//!   1. `Allocator::new(ranges)` — sorted live-range list.
//!   2. For every PC in lower's stream:
//!        * `expire(pc)`  — release ranges whose `end < pc`.
//!        * `assign(id)`  — slot for the def at `pc`.
//!   3. Slots are either physical (`Phys`) or spill (`SpillSlot`).
use alloc::vec::Vec;

use super::scan::{LiveRange, RangeId};
use super::spill::{SpillManager, SpillSlot};
use super::{Phys, Slot, PHYS_POOL};

pub struct Allocator {
    free_phys:    Vec<Phys>,
    /// Active = (RangeId, Slot, end_pc).
    active:       Vec<(RangeId, Slot, u32)>,
    spill:        SpillManager,
    /// Source of truth — kept sorted by `start`.
    ranges:       Vec<LiveRange>,
}

impl Allocator {
    pub fn new(mut ranges: Vec<LiveRange>) -> Self {
        ranges.sort_by_key(|r| r.start);
        Self {
            free_phys: PHYS_POOL.to_vec(),
            active:    Vec::new(),
            spill:     SpillManager::new(),
            ranges,
        }
    }

    /// Release every range whose end is strictly before `pc`.
    pub fn expire(&mut self, pc: u32) {
        let mut i = 0;
        while i < self.active.len() {
            if self.active[i].2 < pc {
                let (id, slot, _) = self.active.swap_remove(i);
                match slot {
                    Slot::Reg(p)   => self.free_phys.push(p),
                    Slot::Spill(_) => self.spill.free(id),
                }
            } else { i += 1; }
        }
    }

    /// Assign a slot to `range`.  Spills the latest-ending active range
    /// when the physical pool is empty.
    pub fn assign(&mut self, range: LiveRange) -> Slot {
        let slot = if let Some(p) = self.free_phys.pop() {
            Slot::Reg(p)
        } else {
            // Spill the active range that ends latest.
            let victim = self.active.iter()
                .max_by_key(|(_, _, end)| *end)
                .map(|(id, _, _)| *id);
            if let Some(victim_id) = victim {
                let victim_slot = self.spill.alloc(victim_id);
                let pos = self.active.iter().position(|(id, _, _)| *id == victim_id).unwrap();
                let (_, _, end) = self.active.remove(pos);
                self.active.push((victim_id, Slot::Spill(victim_slot), end));
                Slot::Reg(self.free_phys.pop().unwrap_or_else(|| {
                    // Pool still empty — spill our new range too.
                    Slot::Spill(self.spill.alloc(range.id));
                    Phys::Rax
                }))
            } else {
                Slot::Spill(self.spill.alloc(range.id))
            }
        };
        self.active.push((range.id, slot.clone_lite(), range.end));
        slot
    }

    pub fn frame_bytes(&self) -> u32 { self.spill.frame_bytes() }

    pub fn ranges(&self) -> &[LiveRange] { &self.ranges }
}

impl Slot {
    fn clone_lite(&self) -> Slot {
        match self {
            Slot::Reg(p)   => Slot::Reg(*p),
            Slot::Spill(s) => Slot::Spill(SpillSlot(s.0)),
        }
    }
}
