//! IRQ-driven reactor.
//!
//! Performance contract on the timer path: `register_timer` and
//! `on_timer_tick` are both **O(log N)** via a binary min-heap keyed on
//! the absolute deadline.  At scale (millions of sleepers / 100 Hz tick)
//! this turns the reactor from a 100 M op/sec hot spot into a few-K
//! op/sec background task.
//!
//! Concurrency contract: every list is protected by an IRQ-safe
//! `SpinLock`.  ISR-side acquisitions race only against task-side
//! registrations on the same CPU — both run with IRQs disabled inside
//! the lock.
use alloc::collections::{BinaryHeap, VecDeque};
use core::cmp::Reverse;
use core::task::Waker;

use crate::arch::x86_64::interrupts::ticks;
use crate::sync::SpinLock;

/// Cap on every wait list — refusing to grow further is the right
/// failure mode at scale (caller should fall back to busy spin / fail).
const SOFT_CAP: usize = 1 << 20;

/// Min-heap entry — `Reverse<(deadline, seq)>` makes the heap pop
/// the *earliest* deadline first.  `seq` breaks deadline ties so two
/// `Sleep`s with identical deadlines remain comparable.
struct TimerEntry {
    key:   Reverse<(u64, u64)>,
    waker: Waker,
}

impl PartialEq for TimerEntry { fn eq(&self, o: &Self) -> bool { self.key.eq(&o.key) } }
impl Eq for TimerEntry {}
impl PartialOrd for TimerEntry {
    fn partial_cmp(&self, o: &Self) -> Option<core::cmp::Ordering> { Some(self.cmp(o)) }
}
impl Ord for TimerEntry {
    fn cmp(&self, o: &Self) -> core::cmp::Ordering { self.key.cmp(&o.key) }
}

#[repr(align(64))]
struct Heap { inner: SpinLock<(BinaryHeap<TimerEntry>, u64)> }
impl Heap {
    const fn new() -> Self {
        Self { inner: SpinLock::new((BinaryHeap::new(), 0)) }
    }
}

#[repr(align(64))]
struct WakerList<T> { inner: SpinLock<VecDeque<T>> }
impl<T> WakerList<T> {
    const fn new() -> Self { Self { inner: SpinLock::new(VecDeque::new()) } }
}

static TIMER:        Heap                 = Heap::new();
static NIC_WAITERS:   WakerList<Waker>    = WakerList::new();
static BLOCK_WAITERS: WakerList<Waker>    = WakerList::new();

/// O(log N) timer registration.
pub fn register_timer(deadline_ticks: u64, w: Waker) {
    let mut g = TIMER.inner.lock();
    if g.0.len() >= SOFT_CAP { log::warn!("[reactor] timer cap"); return; }
    g.1 = g.1.wrapping_add(1);
    let seq = g.1;
    g.0.push(TimerEntry { key: Reverse((deadline_ticks, seq)), waker: w });
}

pub fn register_nic(w: Waker) {
    let mut q = NIC_WAITERS.inner.lock();
    if q.len() >= SOFT_CAP { log::warn!("[reactor] NIC cap"); return; }
    q.push_back(w);
}

pub fn register_block(w: Waker) {
    let mut q = BLOCK_WAITERS.inner.lock();
    if q.len() >= SOFT_CAP { log::warn!("[reactor] block cap"); return; }
    q.push_back(w);
}

/// O(k log N) ISR — `k` = number of timers expiring this tick.
/// Pops only the prefix that has actually fired.
pub fn on_timer_tick() {
    let now = ticks();
    let mut g = TIMER.inner.lock();
    while let Some(top) = g.0.peek() {
        if top.key.0.0 > now { break; }
        let TimerEntry { waker, .. } = g.0.pop().unwrap();
        waker.wake();
    }
}

#[inline]
pub fn wake_nic_waiters()   { drain_into_wake(&NIC_WAITERS); }
#[inline]
pub fn wake_block_waiters() { drain_into_wake(&BLOCK_WAITERS); }

#[inline]
fn drain_into_wake(list: &WakerList<Waker>) {
    let mut q = list.inner.lock();
    while let Some(w) = q.pop_front() { w.wake(); }
}
