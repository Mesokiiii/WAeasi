//! Forward-jump backpatching.
//!
//! Single-pass JIT emits `jmp/jcc rel32` with a placeholder displacement,
//! records the patch site, then fixes it once the target offset is known.
//! Stage 3 supports two label flavours sufficient for `br`/`br_if`/
//! `unreachable` to a function-scope target:
//!
//!   * `LabelId` — opaque handle returned by `new_label`.
//!   * `bind(label, off)` — record the target offset; back-patch every
//!     pending site immediately.
//!
//! Stage 4 will extend with nested labels for `block`/`loop`/`if` once
//! the validator carries control-frame metadata.
use alloc::vec::Vec;

use crate::jit::codebuf::CodeBuffer;
use crate::jit::JitError;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct LabelId(u32);

#[derive(Default)]
pub struct LabelTable {
    /// `Some(off)` once bound.
    bound:   Vec<Option<usize>>,
    /// Pending relocations: `(label, patch_offset, instr_end_offset)`.
    pending: Vec<(LabelId, usize, usize)>,
}

impl LabelTable {
    pub fn new() -> Self { Self::default() }

    /// Reserve a fresh, unbound label.
    pub fn new_label(&mut self) -> LabelId {
        let id = LabelId(self.bound.len() as u32);
        self.bound.push(None);
        id
    }

    /// Record a relocation for a `jmp/jcc rel32` already emitted at
    /// `[patch_off .. patch_off+4]`.  `instr_end` is the offset of the
    /// **next** instruction (used for relative-displacement math).
    pub fn add_pending(&mut self, label: LabelId, patch_off: usize, instr_end: usize) {
        self.pending.push((label, patch_off, instr_end));
    }

    /// Mark `label` as resolved at `target`; immediately patch every
    /// pending site that points here.
    pub fn bind(&mut self, buf: &mut CodeBuffer, label: LabelId, target: usize)
        -> Result<(), JitError>
    {
        let idx = label.0 as usize;
        if idx >= self.bound.len() { return Err(JitError::Unsupported("bad label")); }
        if self.bound[idx].is_some() { return Err(JitError::Unsupported("rebound label")); }
        self.bound[idx] = Some(target);
        // Drain matching pending entries.
        let mut i = 0;
        while i < self.pending.len() {
            if self.pending[i].0 == label {
                let (_, patch, end) = self.pending.swap_remove(i);
                let disp = (target as isize - end as isize) as i32;
                patch_disp32(buf, patch, disp)?;
            } else { i += 1; }
        }
        Ok(())
    }

    /// Verify every label is bound + every relocation patched.
    pub fn finalize(&self) -> Result<(), JitError> {
        if !self.pending.is_empty() {
            return Err(JitError::Unsupported("dangling relocations"));
        }
        for b in &self.bound {
            if b.is_none() { return Err(JitError::Unsupported("unbound label")); }
        }
        Ok(())
    }
}

/// Overwrite the 4-byte displacement in `buf[off .. off+4]` with `disp`.
fn patch_disp32(buf: &mut CodeBuffer, off: usize, disp: i32) -> Result<(), JitError> {
    let bytes = disp.to_le_bytes();
    let base = buf.base().as_mut_ptr::<u8>();
    if off + 4 > buf.cap() { return Err(JitError::Truncated); }
    unsafe {
        for i in 0..4 { *base.add(off + i) = bytes[i]; }
    }
    Ok(())
}
