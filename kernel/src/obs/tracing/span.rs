//! Spans + key/value fields.
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::level::Level;
use super::sink;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SpanId(pub u64);

#[derive(Debug, Clone)]
pub struct Field { pub key: &'static str, pub value: String }

pub struct Span {
    id:         SpanId,
    name:       &'static str,
    level:      Level,
    fields:     Vec<Field>,
    started_tsc:u64,
    closed:     bool,
}

static NEXT_SPAN: AtomicU64 = AtomicU64::new(1);

impl Span {
    /// Start a new span — emits an "enter" event immediately so live
    /// tooling can render an open trace tree.
    pub fn enter(name: &'static str, level: Level) -> Self {
        let id = SpanId(NEXT_SPAN.fetch_add(1, Ordering::Relaxed));
        let started_tsc = crate::arch::x86_64::cpu::rdtsc();
        sink::emit_enter(id, name, level);
        // Pre-size for typical span: 4 fields covers most real
        // production spans without re-alloc.
        Self { id, name, level, fields: Vec::with_capacity(4), started_tsc, closed: false }
    }

    pub fn with<V: Into<String>>(mut self, key: &'static str, value: V) -> Self {
        self.fields.push(Field { key, value: value.into() });
        self
    }

    /// Add a field to an already-running span.
    pub fn record<V: Into<String>>(&mut self, key: &'static str, value: V) {
        self.fields.push(Field { key, value: value.into() });
    }

    /// Emit a free-form event scoped to this span.
    pub fn event(&self, message: &str) {
        sink::emit_event(self.id, self.level, self.name, message, &self.fields);
    }

    pub fn id(&self) -> SpanId { self.id }
}

impl Drop for Span {
    fn drop(&mut self) {
        if self.closed { return; }
        let elapsed = crate::arch::x86_64::cpu::rdtsc().wrapping_sub(self.started_tsc);
        sink::emit_exit(self.id, self.name, self.level, elapsed, &self.fields);
        self.closed = true;
    }
}
