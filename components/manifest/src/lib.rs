//! Component manifest — declarative capability + resource requirements.
//!
//! Builds as ordinary std-lib for host-side tooling
//! (`waeasictl manifest validate`).  Kernel-side TOML parsing happens
//! through the same code paths via the alloc-only re-export.
pub mod parse;
pub mod schema;

pub use parse::{parse, ParseError};
pub use schema::{Manifest, Capabilities, Resources};
