//! Exponential backoff for CAS retry loops.
//!
//! Naked `pause` is fine for short contention but pathological for long
//! contention — every CPU still hammers the cache line.  `Backoff`
//! starts with a few `pause` and doubles up to `MAX_SPINS`, then yields
//! to the executor (which `hlt`s the CPU until the next IRQ).
//!
//! Use in any CAS retry loop where contention is realistic.
use crate::arch::x86_64::cpu::pause;

const MAX_SPINS: u32 = 1 << 10;

pub struct Backoff { step: u32 }

impl Backoff {
    #[inline] pub const fn new() -> Self { Self { step: 1 } }

    /// Spin briefly; doubles wait each call up to `MAX_SPINS`.
    #[inline]
    pub fn spin(&mut self) {
        let n = self.step.min(MAX_SPINS);
        for _ in 0..n { pause(); }
        self.step = self.step.saturating_mul(2);
    }

    /// True once we've reached `MAX_SPINS` — caller may yield instead.
    #[inline] pub fn is_completed(&self) -> bool { self.step >= MAX_SPINS }

    #[inline] pub fn reset(&mut self) { self.step = 1; }
}

impl Default for Backoff { fn default() -> Self { Self::new() } }
