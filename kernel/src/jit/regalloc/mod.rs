//! Single-pass linear-scan register allocator for the JIT.
pub mod scan;
pub mod spill;
pub mod state;

pub use scan::{LiveRange, RangeId};
pub use spill::SpillSlot;
pub use state::Allocator;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Phys { Rax, Rcx, Rdx, R8, R9, R10, R11 }

pub const PHYS_POOL: &[Phys] = &[
    Phys::Rax, Phys::Rcx, Phys::Rdx, Phys::R8, Phys::R9, Phys::R10, Phys::R11,
];

#[derive(Debug)]
pub enum Slot { Reg(Phys), Spill(SpillSlot) }
