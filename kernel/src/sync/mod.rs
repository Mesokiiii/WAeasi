//! Synchronization primitives.
pub mod backoff;
pub mod mutex;
pub mod once;
pub mod spinlock;

pub use backoff::Backoff;
pub use mutex::{Mutex, MutexGuard};
pub use once::Once;
pub use spinlock::{Guard as SpinGuard, SpinLock};

pub fn init() { log::debug!("[sync] primitives ready"); }
