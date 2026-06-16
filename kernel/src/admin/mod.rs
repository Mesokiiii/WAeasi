//! Admin endpoint — line-oriented TCP protocol consumed by `waeasictl`.
//!
//! Wire format: each request is a single line `VERB [args...]\n`; reply
//! is one or more lines terminated by EOF (the kernel closes the
//! connection when the response is complete).
//!
//! Stage-8 commands implemented:
//!   * `LIST`        — `id\tname\tstate\tcaps\tmem` per component
//!   * `VERSION`     — kernel version + Stage banner
//!   * `METRICS`     — Prometheus text exposition
//!   * `DMESG`       — drain kernel ring buffer
//!   * `HEALTH`      — /livez 200 + /readyz code
//!   * `INSPECT id`  — detail snapshot (placeholder)
//!   * `TOP`         — htop-style snapshot (placeholder)
//!
//! Mutating verbs (`KILL`, `RESTART`, `CAP-GRANT`, ...) are accepted
//! but currently reject as `ERR not-implemented` until the per-component
//! state map lands in stage 9.
pub mod handlers;
pub mod protocol;
pub mod server;

pub use server::start as init;
