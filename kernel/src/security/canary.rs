//! Stack canaries — detect stack-smashing in kernel functions before
//! the corrupted return address gets pop'd.
//!
//! We seed a 64-bit random per-CPU value at boot.  Compiler-side support
//! requires `-Z stack-protector=all` (added in `Cargo.toml` build flags
//! when stage 3 lands); meanwhile, security-critical functions can use
//! `Canary::guard()` to spawn a manual canary frame.
use core::sync::atomic::{AtomicU64, Ordering};

use crate::arch::x86_64::per_cpu;

static GLOBAL_SEED: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    let seed = mix_seed();
    GLOBAL_SEED.store(seed, Ordering::Release);
    // Update current CPU's slot.
    per_cpu::current().stack_canary = seed;
    log::debug!("[canary] seeded");
}

fn mix_seed() -> u64 {
    let r = crate::wasi::preview2::random::get_random_u64();
    let t = crate::arch::x86_64::cpu::rdtsc();
    r ^ t.wrapping_mul(0x9E3779B97F4A7C15) ^ 0xBF58476D1CE4E5B9
}

/// Manual canary placed on the local stack frame.
/// Drop checks the value — panic on mismatch.
pub struct Canary { value: u64 }

impl Canary {
    /// SAFETY: must be created at the **top** of a stack frame; the
    /// caller's locals are placed below it.
    #[inline(always)]
    pub fn guard() -> Self {
        Self { value: per_cpu::current().stack_canary }
    }
}

impl Drop for Canary {
    #[inline(always)]
    fn drop(&mut self) {
        let expected = per_cpu::current().stack_canary;
        if self.value != expected {
            panic!("stack canary mismatch: expected {:#x}, got {:#x}",
                   expected, self.value);
        }
    }
}
