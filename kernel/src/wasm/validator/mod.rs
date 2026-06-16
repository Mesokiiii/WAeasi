//! Wasm validator.
//!
//! Implements the algorithmic-cost-bounded validator from the spec:
//! every byte of bytecode is visited exactly once; the type stack is
//! mutated in place.  No allocation in the hot path beyond the type
//! stack itself, which is sized once before validation begins.
//!
//! Validation is what makes Wasm safe — it proves:
//!   * stack neutrality (no underflows at function exit),
//!   * type consistency for every operand,
//!   * structured control flow (every `end` matches a `block`/`loop`/`if`),
//!   * branch targets remain in scope.
//!
//! After validation the interpreter can skip every dynamic check on the
//! hot path → significant perf win.
pub mod ctrl_flow;
pub mod stack;
pub mod type_check;

use crate::wasm::parser::ParsedModule;

#[derive(Debug, PartialEq, Eq)]
pub enum ValidateError {
    StackUnderflow,
    TypeMismatch,
    UnclosedBlock,
    UnknownLocal(u32),
    UnknownGlobal(u32),
    UnknownFunc(u32),
    UnknownType(u32),
    UnknownLabel(u32),
    Unreachable,
    Unsupported(&'static str),
}

/// Validate a previously-parsed module.  Returns `Ok` on success;
/// any failure aborts loading and the audit log records the reason.
pub fn validate(m: &ParsedModule<'_>) -> Result<(), ValidateError> {
    type_check::check_module(m)
}
