//! `wasi:clocks/{wall-clock, monotonic-clock}` host implementation.
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use crate::arch::x86_64::interrupts::ticks;
use crate::arch::x86_64::apic::TICK_HZ;
use crate::sched::reactor;

/// Single source of truth — derived from the LAPIC timer rate so it
/// cannot drift out of sync.
pub const NS_PER_TICK: u64 = 1_000_000_000 / TICK_HZ as u64;

/// `wasi:clocks/monotonic-clock.now` — nanoseconds since boot.
#[inline]
pub fn monotonic_now_ns() -> u64 { ticks() * NS_PER_TICK }

/// `wasi:clocks/wall-clock.now` — Unix epoch nanoseconds.
///
/// Stage 1: same as monotonic.  Stage 2 reads RTC at boot and stores the
/// epoch offset; this function adds it to monotonic.
#[inline]
pub fn wall_now_ns() -> u64 { monotonic_now_ns() }

/// `wasi:clocks/monotonic-clock.subscribe-duration` — async sleep helper.
pub fn sleep_ns(ns: u64) -> Sleep {
    let now = ticks();
    // ceil-divide to avoid sleeping shorter than requested.
    let extra = (ns + NS_PER_TICK - 1) / NS_PER_TICK;
    Sleep { deadline: now + extra.max(1), registered: false }
}

pub struct Sleep {
    deadline: u64,
    registered: bool,
}

impl Future for Sleep {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if ticks() >= self.deadline {
            return Poll::Ready(());
        }
        if !self.registered {
            reactor::register_timer(self.deadline, cx.waker().clone());
            self.registered = true;
        }
        Poll::Pending
    }
}
