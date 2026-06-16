//! Live-range analysis pass.
//!
//! Single forward sweep over the validated Wasm bytecode collects a
//! `LiveRange` per virtual register (= operand-stack slot index).  We
//! treat every `push` as a "def" and every `pop` (consume) as a "use";
//! the range spans `[def_pc, last_use_pc]`.
use alloc::vec::Vec;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct RangeId(pub u32);

#[derive(Copy, Clone, Debug)]
pub struct LiveRange {
    pub id:    RangeId,
    pub start: u32,
    pub end:   u32,
}

pub fn analyze(_code: &[u8]) -> Vec<LiveRange> {
    // Stage-5 scaffold — emits one synthetic range covering the whole
    // function so the rest of the allocator can compile/test.  Stage-6
    // walks every opcode and inserts real ranges keyed by stack depth.
    alloc::vec![ LiveRange { id: RangeId(0), start: 0, end: u32::MAX } ]
}
