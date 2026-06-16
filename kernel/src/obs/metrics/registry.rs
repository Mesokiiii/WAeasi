//! Global metrics registry.
//!
//! Stage-9 hardening:
//!   * `snapshot()` returns an **`Arc<Vec<Entry>>`** — Arc-clone is one
//!     atomic ref-count bump, zero `String` copies, zero `Vec` reallocs.
//!     At trillions of `/metrics` scrapes per node this collapses CPU
//!     spent in the registry lock from O(N entries × N scrapes) to
//!     O(1) per scrape.
//!   * Registration (rare) builds a fresh `Vec`, wraps it in `Arc`,
//!     and replaces the slot under the SpinLock.
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::sync::SpinLock;

use super::counter::Counter;
use super::gauge::Gauge;
use super::histogram::Histogram;

#[derive(Copy, Clone)]
pub enum MetricRef {
    Counter(&'static Counter),
    Gauge  (&'static Gauge),
    Histo  (&'static Histogram),
}

#[derive(Clone)]
pub struct Entry {
    pub name:   String,
    pub help:   &'static str,
    pub metric: MetricRef,
}

static REGISTRY: SpinLock<Option<Arc<Vec<Entry>>>> = SpinLock::new(None);

pub fn init() {
    let mut g = REGISTRY.lock();
    if g.is_none() {
        *g = Some(Arc::new(Vec::with_capacity(64)));
    }
    log::debug!("[metrics] registry ready (Arc-shared snapshots)");
}

fn ensure() -> Arc<Vec<Entry>> {
    let mut g = REGISTRY.lock();
    if g.is_none() { *g = Some(Arc::new(Vec::with_capacity(64))); }
    g.as_ref().unwrap().clone()
}

/// Append a new entry — copy-on-write semantics so existing snapshots
/// are unaffected.  Registration is rare; one allocation per call is fine.
fn push(entry: Entry) {
    let cur = ensure();
    let mut new = (*cur).clone();
    new.push(entry);
    *REGISTRY.lock() = Some(Arc::new(new));
}

pub fn register_counter(name: &str, help: &'static str, c: &'static Counter) {
    push(Entry { name: String::from(name), help, metric: MetricRef::Counter(c) });
}
pub fn register_gauge(name: &str, help: &'static str, g: &'static Gauge) {
    push(Entry { name: String::from(name), help, metric: MetricRef::Gauge(g) });
}
pub fn register_histogram(name: &str, help: &'static str, h: &'static Histogram) {
    push(Entry { name: String::from(name), help, metric: MetricRef::Histo(h) });
}

/// Lock-light snapshot — `Arc` clone is one atomic.  Returns the
/// already-shared backing slice; subsequent registrations get their
/// own Arc, so this snapshot is a stable point-in-time view.
pub fn snapshot() -> Arc<Vec<Entry>> { ensure() }
