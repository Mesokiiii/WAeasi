//! Per-CPU magazine cache in front of the buddy allocator.
//!
//! At trillions of allocations/sec the global buddy lock becomes the
//! bottleneck.  Magazines fix that: each CPU has a small private stack
//! of free frames; alloc/free touch only the local stack on the hot
//! path.  Refill / depletion fall back to the global buddy.
//!
//! Layout: one `Magazine` per CPU, picked via `arch::x86_64::per_cpu`.
use crate::memory::address::PhysAddr;

use super::{alloc, free};

const DEPTH:    usize = 32;
const REFILL:   usize = 16;

pub struct Magazine {
    stack: [PhysAddr; DEPTH],
    len:   usize,
    /// Block order this magazine caches (default: 12 = single 4 KiB frame).
    pub order: u8,
}

impl Magazine {
    pub const fn new(order: u8) -> Self {
        Self { stack: [PhysAddr::new(0); DEPTH], len: 0, order }
    }

    /// Allocate one block — pop from the stack; refill from buddy on miss.
    pub fn alloc(&mut self) -> Option<PhysAddr> {
        if self.len == 0 {
            self.refill();
            if self.len == 0 { return None; }
        }
        self.len -= 1;
        Some(self.stack[self.len])
    }

    /// Return one block — push to the stack; flush half on overflow.
    pub fn free(&mut self, p: PhysAddr) {
        if self.len == DEPTH { self.flush_half(); }
        self.stack[self.len] = p;
        self.len += 1;
    }

    fn refill(&mut self) {
        for _ in 0..REFILL {
            match alloc(self.order) {
                Some(p) => { self.stack[self.len] = p; self.len += 1; }
                None    => return,
            }
        }
    }

    fn flush_half(&mut self) {
        let take = DEPTH / 2;
        for i in 0..take {
            free(self.stack[i], self.order);
        }
        // Compact remaining entries to the front.
        for i in 0..(DEPTH - take) {
            self.stack[i] = self.stack[i + take];
        }
        self.len -= take;
    }
}
