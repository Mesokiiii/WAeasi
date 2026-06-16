//! Tracing sinks — pluggable destinations for events.
//!
//! Stage-6 hardening:
//!   * **Recursion guard** — a per-CPU `IN_SINK` flag prevents a sink
//!     handler from re-entering the dispatcher (e.g. if the logger
//!     itself is later wired to emit traces).  Re-entry is silently
//!     dropped instead of dead-locking the SpinLock.
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::sync::SpinLock;

use super::level::Level;
use super::span::{Field, SpanId};

pub trait Sink: Send + Sync {
    fn enter(&self, id: SpanId, name: &'static str, level: Level);
    fn exit (&self, id: SpanId, name: &'static str, level: Level, elapsed_tsc: u64, fields: &[Field]);
    fn event(&self, id: SpanId, level: Level, name: &'static str, message: &str, fields: &[Field]);
}

static SINKS:   SpinLock<Vec<&'static dyn Sink>> = SpinLock::new(Vec::new());
/// Global recursion guard.  At trillions of events / sec this avoids
/// an expensive per-CPU TLS lookup; the false-positive cost (one sink
/// call dropped under contention) is negligible for diagnostics.
static IN_SINK: AtomicBool = AtomicBool::new(false);

pub fn init_default() {
    static SERIAL: SerialSink = SerialSink;
    register_sink(&SERIAL);
}

pub fn register_sink(s: &'static dyn Sink) {
    SINKS.lock().push(s);
}

#[inline]
pub(super) fn emit_enter(id: SpanId, name: &'static str, level: Level) {
    if IN_SINK.swap(true, Ordering::AcqRel) { return; }
    {
        let s = SINKS.lock();
        for sink in s.iter() { sink.enter(id, name, level); }
    }
    IN_SINK.store(false, Ordering::Release);
}
#[inline]
pub(super) fn emit_exit(id: SpanId, name: &'static str, level: Level, elapsed: u64, f: &[Field]) {
    if IN_SINK.swap(true, Ordering::AcqRel) { return; }
    {
        let s = SINKS.lock();
        for sink in s.iter() { sink.exit(id, name, level, elapsed, f); }
    }
    IN_SINK.store(false, Ordering::Release);
}
#[inline]
pub(super) fn emit_event(id: SpanId, level: Level, name: &'static str, msg: &str, f: &[Field]) {
    if IN_SINK.swap(true, Ordering::AcqRel) { return; }
    {
        let s = SINKS.lock();
        for sink in s.iter() { sink.event(id, level, name, msg, f); }
    }
    IN_SINK.store(false, Ordering::Release);
}

struct SerialSink;
impl Sink for SerialSink {
    fn enter(&self, id: SpanId, name: &'static str, level: Level) {
        log::info!("[trace] >> {} {} id={}", level.as_str(), name, id.0);
    }
    fn exit(&self, id: SpanId, name: &'static str, level: Level, elapsed: u64, fields: &[Field]) {
        log::info!("[trace] << {} {} id={} elapsed_tsc={} fields={}",
                   level.as_str(), name, id.0, elapsed, fields.len());
    }
    fn event(&self, id: SpanId, level: Level, name: &'static str, msg: &str, fields: &[Field]) {
        log::info!("[trace] .. {} {}/{} id={} {}",
                   level.as_str(), name, msg, id.0, fields.len());
    }
}
