//! Bounded async MPMC channel over a `SegQueue`.  Used by components that
//! cooperate via WASI Preview-2 stream resources.
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};
use crossbeam_queue::SegQueue;

pub struct Channel<T> {
    queue: SegQueue<T>,
    pending: AtomicUsize,
}

impl<T> Channel<T> {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            queue: SegQueue::new(),
            pending: AtomicUsize::new(0),
        })
    }
    pub fn send(&self, msg: T) {
        self.queue.push(msg);
        self.pending.fetch_add(1, Ordering::Release);
    }
    pub fn try_recv(&self) -> Option<T> {
        self.queue.pop().inspect(|_| {
            self.pending.fetch_sub(1, Ordering::Release);
        })
    }
    pub fn len(&self) -> usize { self.pending.load(Ordering::Acquire) }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
}
