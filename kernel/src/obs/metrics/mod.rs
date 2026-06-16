//! Prometheus-compatible metrics.
//!
//! Three metric kinds (RFC-equivalent: OpenMetrics):
//!   * `Counter`   — monotonic; only `inc/inc_by`.
//!   * `Gauge`     — set/inc/dec arbitrary direction.
//!   * `Histogram` — bucketed observations + cumulative count + sum.
//!
//! Metrics live in a global `Registry`; an HTTP exporter (`/metrics`)
//! formats them as Prometheus text via `exposition::render`.
pub mod counter;
pub mod exposition;
pub mod gauge;
pub mod histogram;
pub mod registry;

pub use counter::Counter;
pub use exposition::render;
pub use gauge::Gauge;
pub use histogram::Histogram;
pub use registry::{Entry, MetricRef};

pub fn init() {
    registry::init();
}
