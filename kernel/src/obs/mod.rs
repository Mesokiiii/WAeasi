//! Kernel observability — structured tracing + Prometheus-style metrics.
//!
//! Two facets:
//!   * `tracing` — span-based event log; events carry `(level, span_id,
//!     name, key=value, ...)` and feed configurable sinks.
//!   * `metrics` — counter / gauge / histogram + `exposition` formatter
//!     compatible with Prometheus text format (the de-facto monitoring
//!     standard).
//!
//! Both are hot-path-friendly: registration takes a brief lock, recording
//! is `&AtomicU64` ops only.
pub mod metrics;
pub mod tracing;

pub fn init() {
    tracing::init();
    metrics::init();
    log::info!("[obs] tracing + metrics ready");
}
