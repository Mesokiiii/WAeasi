//! Structured tracing.
//!
//! Public API mimics the well-known `tracing` crate (Tokio ecosystem):
//!
//! ```text
//!   let span = Span::enter("tls.handshake", Level::Info)
//!       .with("peer_ip", "10.0.0.1")
//!       .with("session_id", id);
//!   // ... work ...
//!   drop(span);     // emits "exit" event with elapsed ticks
//! ```
//!
//! Events fan out to every registered sink (serial console + ring buffer
//! by default).
pub mod level;
pub mod sink;
pub mod span;

pub use level::Level;
pub use sink::{Sink, register_sink};
pub use span::{Span, SpanId, Field};

pub fn init() {
    sink::init_default();
}
