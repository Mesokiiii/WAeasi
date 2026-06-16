//! HTTP — server-side stack.
//!
//! Stage-5 layout:
//!   * `parser`   — HTTP/1.1 request line + headers (DoS-bounded).
//!   * `response` — HTTP/1.1 response builder.
//!   * `chunked`  — Transfer-Encoding: chunked encode/decode.
//!   * `router`   — exact-match dispatch (legacy).
//!   * `radix`    — radix-tree router with `:param` / `*wildcard`.
//!   * `h2`       — HTTP/2 frame layer + HPACK.
pub mod chunked;
pub mod h2;
pub mod parser;
pub mod radix;
pub mod response;
pub mod router;

pub use parser::{Request, Method};
pub use radix::{RadixRouter, Params};
pub use response::{Response, Status};
pub use router::Router;
