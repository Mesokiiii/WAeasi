//! Operand-type stack used by the validator.
//!
//! In the spec each entry is either a concrete `ValType` or a polymorphic
//! "unknown" placeholder used after unconditional branches.  We model
//! both with a single `OpType` enum.
use alloc::vec::Vec;

use super::ValidateError;
use crate::wasm::parser::types::ValType;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OpType { Val(ValType), Unknown }

impl OpType {
    pub fn val(v: ValType) -> Self { Self::Val(v) }
}

/// Stack with frame markers — used by ctrl_flow to checkpoint depths.
pub struct TypeStack {
    items: Vec<OpType>,
}

impl TypeStack {
    pub fn new() -> Self { Self { items: Vec::with_capacity(64) } }

    pub fn push(&mut self, t: OpType) { self.items.push(t); }

    /// Pop expecting `t`.  `Unknown` matches anything (poly stack).
    pub fn pop_expect(&mut self, t: ValType) -> Result<(), ValidateError> {
        match self.items.pop() {
            None => Err(ValidateError::StackUnderflow),
            Some(OpType::Unknown) => Ok(()),
            Some(OpType::Val(v)) if v == t => Ok(()),
            Some(_) => Err(ValidateError::TypeMismatch),
        }
    }

    /// Pop any value — returns its type.
    pub fn pop_any(&mut self) -> Result<OpType, ValidateError> {
        self.items.pop().ok_or(ValidateError::StackUnderflow)
    }

    pub fn len(&self) -> usize { self.items.len() }
    pub fn truncate(&mut self, depth: usize) { self.items.truncate(depth); }

    /// Mark the rest of the current branch as polymorphic.  Used after
    /// `unreachable`, `br`, `return` — anything that yields no operand.
    pub fn poly(&mut self) {
        self.items.push(OpType::Unknown);
    }
}
