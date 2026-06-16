//! Per-CPU multi-priority ready queue with work-stealing.
//!
//! Stores `Arc<Task>` directly — there is no global task table.  A task's
//! lifetime is tied to the Arc held by (a) the queue while it is pending
//! and (b) the executor while it is being polled.  When both drop, the
//! task is freed.
//!
//! Layout (logical):
//! ```text
//!   CPU0 local  : [ kernel | high | normal | idle ]
//!   CPU1 local  : [ kernel | high | normal | idle ]
//!   ...
//!   global injector: [ kernel | high | normal | idle ]
//! ```
//!
//! Each `SegQueue` lives on its own cache line via `#[repr(align(64))]`.
//! Stage 1 instantiates a single CPU; the API is already SMP-shaped.
use alloc::sync::Arc;
use alloc::vec::Vec;
use crossbeam_queue::SegQueue;

use super::priority::Priority;
use super::task::Task;

#[repr(align(64))]
struct PaddedQueue(SegQueue<Arc<Task>>);
impl PaddedQueue { fn new() -> Self { Self(SegQueue::new()) } }

pub struct ReadyQueue {
    local:    Vec<[PaddedQueue; Priority::COUNT]>,
    injector: [PaddedQueue; Priority::COUNT],
}

impl ReadyQueue {
    pub fn new(cpu_count: usize) -> Self {
        let mut local = Vec::with_capacity(cpu_count);
        for _ in 0..cpu_count {
            local.push([
                PaddedQueue::new(), PaddedQueue::new(),
                PaddedQueue::new(), PaddedQueue::new(),
            ]);
        }
        Self {
            local,
            injector: [
                PaddedQueue::new(), PaddedQueue::new(),
                PaddedQueue::new(), PaddedQueue::new(),
            ],
        }
    }

    /// Push to the current CPU's local queue (zero contention).
    #[inline]
    pub fn push_local(&self, cpu: usize, task: Arc<Task>) {
        let prio = task.priority.index();
        self.local[cpu][prio].0.push(task);
    }

    /// Push to the global injector — used from IRQs / cross-CPU spawns.
    #[inline]
    pub fn push_global(&self, task: Arc<Task>) {
        let prio = task.priority.index();
        self.injector[prio].0.push(task);
    }

    /// Highest-priority pop for `cpu`: local → injector → steal.
    pub fn pop(&self, cpu: usize) -> Option<Arc<Task>> {
        for bucket in self.local[cpu].iter().rev() {
            if let Some(t) = bucket.0.pop() { return Some(t); }
        }
        for bucket in self.injector.iter().rev() {
            if let Some(t) = bucket.0.pop() { return Some(t); }
        }
        let n = self.local.len();
        for i in 1..n {
            let victim = (cpu + i) % n;
            for bucket in self.local[victim].iter().rev() {
                if let Some(t) = bucket.0.pop() { return Some(t); }
            }
        }
        None
    }

    pub fn len(&self) -> usize {
        let l: usize = self.local.iter().flat_map(|c| c.iter()).map(|q| q.0.len()).sum();
        let i: usize = self.injector.iter().map(|q| q.0.len()).sum();
        l + i
    }

    pub fn is_empty(&self) -> bool { self.len() == 0 }
}

pub type SharedQueue = Arc<ReadyQueue>;
