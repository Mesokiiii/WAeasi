//! Thin wrapper around `spin::Mutex` so call sites depend on
//! `kernel::sync::Mutex` instead of an external crate.
pub use spin::Mutex;
pub use spin::MutexGuard;
