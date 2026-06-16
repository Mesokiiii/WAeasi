//! IRQ-safe spinlock.
//!
//! Two locking modes:
//!   * `lock()`        — atomically masks IRQs **before** taking the lock,
//!                       so an ISR on the same CPU cannot deadlock.
//!   * `lock_no_irq()` — caller must already be in IRQ-off context (panic
//!                       handler, ISR, or under another `lock`).
//!
//! Spinning uses `pause` + cache-friendly relaxed reads.
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

use crate::arch::x86_64::cpu::{self, IrqState};

#[repr(align(64))] // dedicated cache line — avoid false sharing
pub struct SpinLock<T: ?Sized> {
    locked: AtomicBool,
    data:   UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: ?Sized + Send> Sync for SpinLock<T> {}

pub struct Guard<'a, T: ?Sized> {
    lock:  &'a SpinLock<T>,
    saved: IrqState,
    irq:   bool,
}

impl<T> SpinLock<T> {
    pub const fn new(t: T) -> Self {
        Self { locked: AtomicBool::new(false), data: UnsafeCell::new(t) }
    }
}

impl<T: ?Sized> SpinLock<T> {
    /// IRQ-safe lock — used by everything that **may** be touched from an ISR.
    #[inline]
    pub fn lock(&self) -> Guard<'_, T> {
        let saved = cpu::save_and_disable_interrupts();
        self.spin_acquire();
        Guard { lock: self, saved, irq: true }
    }

    /// Lock without touching the interrupt flag — caller asserts IRQs are
    /// already off (or that this lock is never shared with an ISR).
    #[inline]
    pub fn lock_no_irq(&self) -> Guard<'_, T> {
        self.spin_acquire();
        Guard { lock: self, saved: cpu::save_and_disable_interrupts(), irq: false }
    }

    /// Best-effort try-lock — never spins.  Returns `None` on contention.
    #[inline]
    pub fn try_lock(&self) -> Option<Guard<'_, T>> {
        let saved = cpu::save_and_disable_interrupts();
        if self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            Some(Guard { lock: self, saved, irq: true })
        } else {
            cpu::restore_interrupts(saved);
            None
        }
    }

    #[inline(always)]
    fn spin_acquire(&self) {
        // Test-and-test-and-set: read first to avoid hammering the cache line.
        loop {
            if self.locked
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return;
            }
            while self.locked.load(Ordering::Relaxed) {
                cpu::pause();
            }
        }
    }
}

impl<'a, T: ?Sized> Deref for Guard<'a, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &T { unsafe { &*self.lock.data.get() } }
}
impl<'a, T: ?Sized> DerefMut for Guard<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.lock.data.get() } }
}
impl<'a, T: ?Sized> Drop for Guard<'a, T> {
    #[inline(always)]
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
        if self.irq {
            cpu::restore_interrupts(self.saved);
        }
    }
}
