//! Per-call activation frame.  `locals` are 8-byte `Cell`s — same size
//! as values on the operand stack so `local.get/set` is a single move.
use alloc::vec::Vec;

use super::value::Cell;

pub struct Frame<'a> {
    pub code:        &'a [u8],
    pub pc:          usize,
    pub locals:      Vec<Cell>,
    pub stack_base:  usize,    // ValueStack `top` at frame entry
    pub n_results:   usize,
}

pub struct CallStack<'a> {
    frames: Vec<Frame<'a>>,
}

impl<'a> CallStack<'a> {
    pub fn new() -> Self { Self { frames: Vec::with_capacity(16) } }
    pub fn push(&mut self, f: Frame<'a>) { self.frames.push(f); }
    pub fn pop(&mut self) -> Option<Frame<'a>> { self.frames.pop() }
    pub fn top(&mut self) -> Option<&mut Frame<'a>> { self.frames.last_mut() }
    pub fn depth(&self) -> usize { self.frames.len() }
    pub fn is_empty(&self) -> bool { self.frames.is_empty() }
}
