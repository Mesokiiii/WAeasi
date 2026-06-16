//! Gauge — bidirectional 64-bit atomic value with set/inc/dec.
//!
//! Stored as `i64` bits in an `AtomicU64` so we can express negative
//! deltas without `compare_exchange` loops on the fast path.
use core::sync::atomic::{AtomicU64, Ordering};

pub struct Gauge { value: AtomicU64 }

impl Gauge {
    pub const fn new() -> Self { Self { value: AtomicU64::new(0) } }

    #[inline(always)]
    pub fn set(&self, v: i64) { self.value.store(v as u64, Ordering::Relaxed); }

    #[inline(always)]
    pub fn inc(&self)         { self.add(1); }

    #[inline(always)]
    pub fn dec(&self)         { self.add(-1); }

    #[inline(always)]
    pub fn add(&self, delta: i64) {
        if delta >= 0 {
            self.value.fetch_add(delta as u64, Ordering::Relaxed);
        } else {
            self.value.fetch_sub((-delta) as u64, Ordering::Relaxed);
        }
    }

    #[inline(always)]
    pub fn get(&self) -> i64 { self.value.load(Ordering::Relaxed) as i64 }
}
