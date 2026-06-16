//! `call_indirect` — table-based indirect calls.
//!
//! Spec: pop `i32 idx`; look up `table[idx]` (must be a `funcref`);
//! type-check against the declared `type_idx`; transfer control.
//!
//! Stage 3 wires the runtime-side machinery; the interpreter dispatcher
//! routes opcode 0x11 here.  JIT lowering arrives in stage 4.
use alloc::vec::Vec;

use crate::wasm::parser::types::FuncType;
use crate::wasm::trap::Trap;

/// A populated function table — produced by combining
/// `parsed_module.tables` + `parsed_module.elements`.
pub struct FuncTable {
    pub entries: Vec<Option<FuncRef>>,
}

#[derive(Copy, Clone, Debug)]
pub struct FuncRef {
    pub func_idx: u32,
    pub type_idx: u32,
}

impl FuncTable {
    pub fn new(size: u32) -> Self {
        Self { entries: alloc::vec![None; size as usize] }
    }

    /// Populate from the parsed Element section + Table section.
    pub fn fill(
        &mut self,
        offset: u32,
        funcs:  &[u32],
        type_of: impl Fn(u32) -> Option<u32>,
    ) {
        for (i, &fidx) in funcs.iter().enumerate() {
            let pos = offset as usize + i;
            if pos >= self.entries.len() { break; }
            if let Some(tidx) = type_of(fidx) {
                self.entries[pos] = Some(FuncRef { func_idx: fidx, type_idx: tidx });
            }
        }
    }

    /// Resolve `idx`, checking it matches the caller's expected type.
    /// Returns `Trap::OutOfBounds` for out-of-range, `Trap::HostError`
    /// for type mismatch (Wasm spec calls this "indirect call type
    /// mismatch").
    pub fn resolve<'a>(
        &self,
        idx:  u32,
        expected_type: &FuncType,
        type_of_idx: impl Fn(u32) -> Option<&'a FuncType>,
    ) -> Result<u32, Trap> {
        let entry = self.entries.get(idx as usize)
            .and_then(|s| s.as_ref())
            .ok_or(Trap::OutOfBounds)?;
        let actual = type_of_idx(entry.type_idx).ok_or(Trap::HostError)?;
        if !type_eq(actual, expected_type) { return Err(Trap::HostError); }
        Ok(entry.func_idx)
    }
}

fn type_eq(a: &FuncType, b: &FuncType) -> bool {
    a.params == b.params && a.results == b.results
}
