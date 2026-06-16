//! Function prologue / epilogue.
//!
//! ABI used by JIT'd Wasm functions:
//!   * `r15` — base of the linear memory.
//!   * `r14` — fuel counter (decrements each opcode; underflow traps).
//!   * `rbp` — frame pointer; locals at `[rbp - 8*idx]`.
//!   * `rsp` — operand stack pointer (we use the native stack as the
//!             Wasm operand stack — push/pop are 1 instruction each).
//!
//! `prologue` saves callee-save registers + reserves `n_locals * 8` bytes;
//! `epilogue` reverses the sequence and `ret`s.
use super::call::{pop, push};
use super::mov::mov_rr;
use super::reg::Reg;
use crate::jit::codebuf::CodeBuffer;
use crate::jit::JitError;

/// `push rbp; mov rbp, rsp; sub rsp, frame_size`
pub fn prologue(buf: &mut CodeBuffer, n_locals: u32) -> Result<(), JitError> {
    push(buf, Reg::Rbp)?;
    mov_rr(buf, Reg::Rbp, Reg::Rsp)?;
    if n_locals > 0 {
        // sub rsp, n_locals*8
        super::alu::add_imm32(buf, Reg::Rsp, -((n_locals as i32) * 8))?;
    }
    Ok(())
}

/// `mov rsp, rbp; pop rbp; ret`
pub fn epilogue(buf: &mut CodeBuffer) -> Result<(), JitError> {
    mov_rr(buf, Reg::Rsp, Reg::Rbp)?;
    pop(buf, Reg::Rbp)?;
    super::call::ret(buf)
}
