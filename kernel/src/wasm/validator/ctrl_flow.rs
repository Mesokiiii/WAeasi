//! Control-flow frame stack.  One frame per `block`/`loop`/`if`.
//!
//! A `Frame` records the enclosing label's expected types and the type
//! stack depth at entry, so a `br` can pop down to it deterministically.
use alloc::vec::Vec;

use super::ValidateError;
use crate::wasm::parser::types::ValType;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameKind { Block, Loop, If }

#[derive(Debug)]
pub struct Frame {
    pub kind:        FrameKind,
    pub stack_depth: usize,
    pub branch_ty:   Option<ValType>, // single-result MVP
    pub end_ty:      Option<ValType>,
    pub unreachable: bool,
}

#[derive(Default)]
pub struct ControlStack { frames: Vec<Frame> }

impl ControlStack {
    pub fn new() -> Self { Self { frames: Vec::with_capacity(16) } }

    pub fn push(&mut self, f: Frame) { self.frames.push(f); }

    pub fn pop(&mut self) -> Result<Frame, ValidateError> {
        self.frames.pop().ok_or(ValidateError::UnclosedBlock)
    }

    pub fn label(&self, depth: u32) -> Result<&Frame, ValidateError> {
        let idx = self.frames.len().checked_sub(1 + depth as usize)
            .ok_or(ValidateError::UnknownLabel(depth))?;
        Ok(&self.frames[idx])
    }

    pub fn last_mut(&mut self) -> Option<&mut Frame> { self.frames.last_mut() }
    pub fn len(&self) -> usize { self.frames.len() }
    pub fn is_empty(&self) -> bool { self.frames.is_empty() }
}
