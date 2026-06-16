//! Custom `Waker` — eliminates spurious polls.
//!
//! Wake protocol:
//!   * If task is IDLE          → CAS to QUEUED, push Arc<Task> to queue.
//!   * If task is QUEUED        → no-op (already pending).
//!   * If task is RUNNING       → set WAKE bit; the executor will requeue
//!                                the task after the current poll returns.
//!   * If task is DONE          → no-op.
//!
//! This reduces queue traffic from O(wake_calls) to O(state_transitions).
use alloc::sync::Arc;
use core::task::{RawWaker, RawWakerVTable, Waker};
use core::sync::atomic::Ordering;

use super::queue::SharedQueue;
use super::task::{
    Task, STATE_IDLE, STATE_QUEUED, STATE_RUNNING, STATE_WAKE, STATE_DONE,
};

pub struct WakerData {
    pub task:  Arc<Task>,
    pub queue: SharedQueue,
}

const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop_data);

pub fn create(data: Arc<WakerData>) -> Waker {
    let raw = RawWaker::new(Arc::into_raw(data) as *const (), &VTABLE);
    unsafe { Waker::from_raw(raw) }
}

#[inline]
fn schedule(data: &Arc<WakerData>) {
    let task = &data.task;
    loop {
        let cur = task.state.load(Ordering::Acquire);
        if cur & STATE_DONE != 0 { return; }

        // Already queued? nothing to do.
        if cur & STATE_QUEUED != 0 { return; }

        // Currently being polled? set WAKE bit; the executor will requeue.
        if cur & STATE_RUNNING != 0 {
            let new = cur | STATE_WAKE;
            if task.state
                .compare_exchange_weak(cur, new, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            { return; }
            continue;
        }

        // Idle → mark QUEUED and inject.
        if task.state
            .compare_exchange_weak(STATE_IDLE, STATE_QUEUED,
                                   Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            data.queue.push_global(task.clone());
            return;
        }
    }
}

unsafe fn clone(p: *const ()) -> RawWaker {
    let arc = Arc::from_raw(p as *const WakerData);
    let cloned = arc.clone();
    let _ = Arc::into_raw(arc); // keep refcount intact
    RawWaker::new(Arc::into_raw(cloned) as *const (), &VTABLE)
}

unsafe fn wake(p: *const ()) {
    let arc = Arc::from_raw(p as *const WakerData);
    schedule(&arc);
    drop(arc);
}

unsafe fn wake_by_ref(p: *const ()) {
    let arc = Arc::from_raw(p as *const WakerData);
    schedule(&arc);
    let _ = Arc::into_raw(arc); // restore refcount
}

unsafe fn drop_data(p: *const ()) {
    drop(Arc::from_raw(p as *const WakerData));
}
