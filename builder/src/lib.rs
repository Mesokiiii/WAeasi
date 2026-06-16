//! waeasi-builder — language-agnostic build pipeline.
//!
//! Drives any pre-built Wasm Component (from TinyGo, jco, componentize-py,
//! Rust+wit-bindgen, etc.) through the canonical WAeasi packaging
//! sequence:
//!
//! ```text
//!   *.wasm  →  wizer  →  split  →  manifest  →  sign  →  bundle
//! ```
//!
//! The TS and Python SDKs each ship their own thin pipeline driver
//! because they need to invoke their language toolchain (jco /
//! componentize-py) before the binary stages.  This crate is what
//! drives Go (TinyGo emits a finished component in one step) and
//! anyone who already has a `.wasm` they want packaged.
//!
//! Public API:
//!
//! ```ignore
//! use waeasi_builder::{Pipeline, Manifest, KeySource};
//!
//! let mut p = Pipeline::new("dist".into())?;
//! p.run(
//!     "raw.wasm",
//!     Manifest::from_path("waeasi.toml")?,
//!     KeySource::Env("WAEASI_SIGN_KEY".into()),
//! )?;
//! ```

pub mod digest;
pub mod error;
pub mod fs;
pub mod manifest;
pub mod pipeline;
pub mod stages;

pub use error::{BuildError, Result};
pub use manifest::{Manifest, ManifestSpec, Right, World};
pub use pipeline::{KeySource, Pipeline, PipelineOptions, Report, StageTiming};
