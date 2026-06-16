//! `Task` — the unit of work the executor schedules.
//!
//! Hot-path invariants (enforced by `state` atomic):
//!
//! ```text
//!         spawn / wake
//!   IDLE ─────────────────► QUEUED ────────────► RUNNING
//!    ▲                                              │
//!    │                                  Pending     │
//!    └──────────────────────────────────────────────┘
//!                          │
//!                          ▼ (woken during poll)
//!                       RERUN — re-queued after poll completes
//! ```
//!
//! Wakers do **NOT** push the task to the queue if it is already QUEUED or
//! RUNNING — they merely set the `WAKE` bit, which the executor observes
//! after each `poll`.  This eliminates spurious polls and queue churn.
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use core::task::{Context, Poll};

use super::priority::Priority;

/// Globally unique task id — useful for tracing.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TaskId(pub u64);

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
impl TaskId {
    fn new() -> Self { Self(NEXT_ID.fetch_add(1, Ordering::Relaxed)) }
}

// State machine bits.
pub(crate) const STATE_IDLE:    u8 = 0b0000;
pub(crate) const STATE_QUEUED:  u8 = 0b0001;
pub(crate) const STATE_RUNNING: u8 = 0b0010;
pub(crate) const STATE_WAKE:    u8 = 0b0100; // re-arm flag
pub(crate) const STATE_DONE:    u8 = 0b1000;

/// Pinned future + scheduling metadata.  A `Task` is always handled via
/// `Arc<Task>`; cloning the Arc is the only safe way to share it across
/// the run queue and its waker.
pub struct Task {
    pub id:       TaskId,
    pub priority: Priority,
    pub(crate) state: AtomicU8,
    future: UnsafeCell<Pin<Box<dyn Future<Output = ()> + Send + 'static>>>,
}

// SAFETY: the future cell is mutated only inside `poll`, which is gated by
// the `RUNNING` bit of `state` (CAS'd atomically).  Two threads can never
// observe `RUNNING` simultaneously.
unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl Task {
    pub fn new<F>(future: F) -> Arc<Self>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Arc::new(Self {
            id: TaskId::new(),
            priority: Priority::Normal,
            state: AtomicU8::new(STATE_IDLE),
            future: UnsafeCell::new(Box::pin(future)),
        })
    }

    pub fn with_priority<F>(future: F, prio: Priority) -> Arc<Self>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Arc::new(Self {
            id: TaskId::new(),
            priority: prio,
            state: AtomicU8::new(STATE_IDLE),
            future: UnsafeCell::new(Box::pin(future)),
        })
    }

    /// Drive the future once.  Caller must have observed (and CAS-set) the
    /// `RUNNING` bit before invoking this.  Returns whether the task is now
    /// done.  After this call, the `RUNNING` bit is cleared and any
    /// `WAKE` re-arm is observable.
    pub(crate) unsafe fn poll(self: &Arc<Self>, cx: &mut Context<'_>) -> bool {
        let fut = &mut *self.future.get();
        match fut.as_mut().poll(cx) {
            Poll::Ready(()) => {
                self.state.store(STATE_DONE, Ordering::Release);
                true
            }
            Poll::Pending => false,
        }
    }
}
