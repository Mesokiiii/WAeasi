//! Async-first scheduler.
//!
//! Every Wasm component is a `Future`; the scheduler is what makes
//! "thousands of components in one address space" practical — no context
//! switches, no kernel/user boundary, only cooperative multitasking driven
//! by I/O readiness.
pub mod executor;
pub mod priority;
pub mod queue;
pub mod reactor;
pub mod task;
pub mod waker;

/// Spawn the boot service that loads & runs every component shipped under
/// `components/` at compile time.
pub fn bootstrap() {
    log::info!("[sched] bootstrapping");
    executor::Executor::global().spawn(task::Task::new(boot_service()));
}

async fn boot_service() {
    log::info!("[sched] boot service running — loading components");
    if let Err(e) = crate::wasm::engine::load_builtin_components().await {
        log::error!("[sched] failed to load builtin components: {:?}", e);
    }
}
