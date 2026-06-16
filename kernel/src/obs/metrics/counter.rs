//! Monotonic counter — `&AtomicU64` under the hood.  Sub-nanosecond on
//! the hot path, no contention if every CPU has its own `Counter`.
use core::sync::atomic::{AtomicU64, Ordering};

pub struct Counter { value: AtomicU64 }

impl Counter {
    pub const fn new() -> Self { Self { value: AtomicU64::new(0) } }

    #[inline(always)]
    pub fn inc(&self) { self.inc_by(1); }

    #[inline(always)]
    pub fn inc_by(&self, n: u64) { self.value.fetch_add(n, Ordering::Relaxed); }

    #[inline(always)]
    pub fn get(&self) -> u64 { self.value.load(Ordering::Relaxed) }
}
