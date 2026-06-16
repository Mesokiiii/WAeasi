//! Prometheus text exposition (Prometheus 0.0.4 / OpenMetrics-1.0).
//!
//! Stage-9 hardening:
//!   * Iterates an `Arc<Vec<Entry>>` snapshot — registry SpinLock is
//!     held only for the Arc-clone (one atomic), never during
//!     formatting.
//!   * Capacity heuristic accounts for histogram bloat (~256 B/entry).
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;

use super::registry::{snapshot, MetricRef};

pub fn render() -> String {
    let entries = snapshot();
    let est = entries.len() * 256;
    let mut out = String::with_capacity(est);
    for entry in entries.iter() {
        match entry.metric {
            MetricRef::Counter(c) => {
                let _ = writeln!(out, "# HELP {} {}", entry.name, entry.help);
                let _ = writeln!(out, "# TYPE {} counter", entry.name);
                let _ = writeln!(out, "{} {}", entry.name, c.get());
            }
            MetricRef::Gauge(g) => {
                let _ = writeln!(out, "# HELP {} {}", entry.name, entry.help);
                let _ = writeln!(out, "# TYPE {} gauge", entry.name);
                let _ = writeln!(out, "{} {}", entry.name, g.get());
            }
            MetricRef::Histo(h) => {
                let (sum, total, buckets) = h.snapshot();
                let _ = writeln!(out, "# HELP {} {}", entry.name, entry.help);
                let _ = writeln!(out, "# TYPE {} histogram", entry.name);
                for (b, n) in &buckets {
                    let _ = writeln!(out, "{}_bucket{{le=\"{}\"}} {}", entry.name, b, n);
                }
                let _ = writeln!(out, "{}_bucket{{le=\"+Inf\"}} {}", entry.name, total);
                let _ = writeln!(out, "{}_sum {}",   entry.name, sum);
                let _ = writeln!(out, "{}_count {}", entry.name, total);
            }
        }
    }
    out
}

pub const DEFAULT_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

pub fn render_to_bytes() -> Vec<u8> { render().into_bytes() }
