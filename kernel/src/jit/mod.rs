//! Baseline JIT — single-pass Wasm → x86_64 native code.
pub mod asm;
pub mod codebuf;
pub mod labels;
pub mod lower;
pub mod regalloc;
pub mod runtime;

pub use codebuf::CodeBuffer;
pub use labels::{LabelId, LabelTable};
pub use lower::compile_function;

#[derive(Debug)]
pub enum JitError {
    OutOfMemory,
    Unsupported(&'static str),
    Truncated,
}
