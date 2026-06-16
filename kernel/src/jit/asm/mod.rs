//! x86_64 instruction emitter — typed enough to avoid REX-byte mistakes.
//!
//! Each module emits a focused subset:
//!   * `mov`     — register/immediate/memory moves.
//!   * `alu`     — add/sub/and/or/xor/shl/shr.
//!   * `branch`  — jmp/jcc/cmp.
//!   * `call`    — call/ret + System V calling convention.
//!   * `prologue` — function entry/exit boilerplate.
pub mod alu;
pub mod branch;
pub mod call;
pub mod mov;
pub mod prologue;
pub mod reg;

pub use reg::Reg;
