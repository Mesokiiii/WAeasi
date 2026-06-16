//! Inter-Wasm IPC.  Two flavours:
//!   * `channel` — typed async channels between components,
//!   * `message` — opaque byte-buffer messages routed by capability.
pub mod channel;
pub mod message;
