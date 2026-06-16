//! Cooperative executor — per-CPU loop, lock-free hot path,
//! work-stealing across CPUs.
//!
//! Initialization contract:
//!   * `init_pool(cpu_count)` MUST be called exactly once at boot, before
//!     any `spawn`/`global`/`for_cpu` access.
//!   * Subsequent calls to `init_pool` are silently ignored (idempotent).
//!   * `global()` and `for_cpu(0)` resolve to the BSP's executor.
//!   * Calling `global()` before `init_pool` panics — no silent
//!     single-CPU fallback that swallows SMP startup.
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use core::task::Context;
use spin::Once;

use super::queue::{ReadyQueue, SharedQueue};
use super::task::{Task, STATE_QUEUED, STATE_RUNNING, STATE_WAKE, STATE_IDLE};
use super::waker::{create as create_waker, WakerData};
use crate::arch;

const MAX_CPUS: usize = 64;

pub struct Executor {
    queue:  SharedQueue,
    cpu_id: usize,
}

struct Pool {
    queue:     SharedQueue,
    executors: Vec<Executor>,
}

static POOL: Once<Pool> = Once::new();

impl Executor {
    /// Initialize the executor pool for `cpu_count` CPUs (1..=MAX_CPUS).
    /// Idempotent; subsequent calls are ignored.
    pub fn init_pool(cpu_count: usize) {
        let cpu_count = cpu_count.max(1).min(MAX_CPUS);
        POOL.call_once(|| {
            let queue = Arc::new(ReadyQueue::new(cpu_count));
            let executors = (0..cpu_count)
                .map(|i| Executor { queue: queue.clone(), cpu_id: i })
                .collect();
            log::info!("[executor] pool: {} CPU(s)", cpu_count);
            Pool { queue, executors }
        });
    }

    /// BSP-side accessor.  Panics if `init_pool` was not called.
    pub fn global() -> &'static Executor {
        Self::for_cpu(0)
    }

    pub fn for_cpu(cpu_id: u32) -> &'static Executor {
        let pool = POOL.get().expect("Executor::init_pool not called");
        pool.executors.get(cpu_id as usize)
            .expect("cpu_id out of range — bigger than pool")
    }

    pub fn cpu_count() -> usize {
        POOL.get().map(|p| p.executors.len()).unwrap_or(0)
    }

    /// Enqueue a fresh task on this CPU's local queue.
    pub fn spawn(&self, task: Arc<Task>) {
        task.state.store(STATE_QUEUED, Ordering::Release);
        self.queue.push_local(self.cpu_id, task);
    }

    /// Run forever.
    pub fn run(&'static self) -> ! {
        log::info!("[executor] CPU {} entering main loop", self.cpu_id);
        loop {
            while let Some(task) = self.queue.pop(self.cpu_id) {
                self.poll_one(task);
            }
            arch::halt();
        }
    }

    fn poll_one(&'static self, task: Arc<Task>) {
        // QUEUED → RUNNING.  Any concurrent wake will set WAKE on top
        // of RUNNING; the post-poll loop observes it.
        let _prev = task.state.swap(STATE_RUNNING, Ordering::AcqRel);

        let waker_data = Arc::new(WakerData {
            task:  task.clone(), queue: self.queue.clone(),
        });
        let waker = create_waker(waker_data);
        let mut cx = Context::from_waker(&waker);

        let done = unsafe { task.poll(&mut cx) };
        if done { return; }

        loop {
            let cur = task.state.load(Ordering::Acquire);
            let wake = cur & STATE_WAKE != 0;
            let new = if wake { STATE_QUEUED } else { STATE_IDLE };
            if task.state
                .compare_exchange_weak(cur, new, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                if wake { self.queue.push_local(self.cpu_id, task); }
                return;
            }
        }
    }

    pub fn pending(&self) -> usize { self.queue.len() }
}
