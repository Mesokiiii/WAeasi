//! Pre-sized operand stack.
//!
//! Validator proves the maximum stack depth per function.  We allocate
//! exactly that — no `Vec::push` / capacity doubling on the hot path,
//! no bounds check on pop (debug-asserted).
//!
//! Hot-path helpers are `#[inline(always)]` so LLVM lowers `pop_i32 +
//! pop_i32 + push(add)` into three load/store + add — no calls.
use alloc::vec::Vec;

use super::value::Cell;

pub struct ValueStack {
    storage: Vec<Cell>, // logical len; `top` is current depth
    top:     usize,
    cap:     usize,
}

impl ValueStack {
    /// Pre-allocate exactly `max_depth` slots.  Stage 3 will replace
    /// `Vec` with a raw kernel-owned page so no allocator round-trips.
    pub fn with_capacity(max_depth: usize) -> Self {
        let storage = alloc::vec![Cell(0); max_depth];
        Self { storage, top: 0, cap: max_depth }
    }

    #[inline(always)]
    pub fn push(&mut self, v: Cell) {
        debug_assert!(self.top < self.cap, "stack overflow — validator missed it");
        unsafe { *self.storage.get_unchecked_mut(self.top) = v; }
        self.top += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Cell {
        debug_assert!(self.top > 0, "stack underflow — validator missed it");
        self.top -= 1;
        unsafe { *self.storage.get_unchecked(self.top) }
    }

    #[inline(always)] pub fn pop_i32(&mut self) -> i32 { self.pop().as_i32() }
    #[inline(always)] pub fn pop_i64(&mut self) -> i64 { self.pop().as_i64() }
    #[inline(always)] pub fn pop_u32(&mut self) -> u32 { self.pop().as_u32() }

    #[inline(always)]
    pub fn peek(&self, depth: usize) -> Cell {
        debug_assert!(depth < self.top);
        unsafe { *self.storage.get_unchecked(self.top - 1 - depth) }
    }

    /// View the live slots as a slice — used by the dispatcher to
    /// return the final stack without pop/push reverse-shuffle.
    #[inline(always)]
    pub fn as_slice(&self) -> &[Cell] {
        &self.storage[..self.top]
    }

    #[inline(always)] pub fn len(&self) -> usize { self.top }
    #[inline(always)] pub fn truncate(&mut self, n: usize) {
        debug_assert!(n <= self.top);
        self.top = n;
    }
}
