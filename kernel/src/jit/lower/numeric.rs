//! Numeric lowering.
//!
//! Stage 3 baseline: operands are kept on the **native CPU stack**
//! (one push/pop per Wasm push/pop).  Stage 4 register allocator will
//! pin the top-of-stack into `rax`/`rcx` to remove most push/pop pairs.
use crate::jit::asm::alu::{self, Logic};
use crate::jit::asm::call::{pop, push};
use crate::jit::asm::mov;
use crate::jit::asm::reg::Reg;
use crate::jit::codebuf::CodeBuffer;
use crate::jit::JitError;
use crate::wasm::parser::leb_i32;

#[derive(Copy, Clone, Debug)]
pub enum Bin { Add, Sub, Mul, And, Or, Xor }

/// `i32.const c` → `mov rax, c; push rax`
pub fn i32_const(buf: &mut CodeBuffer, bytes: &[u8], p: &mut usize) -> Result<(), JitError> {
    let c = leb_i32(bytes, p).map_err(|_| JitError::Truncated)?;
    mov::mov_imm64(buf, Reg::Rax, c as i64 as u64)?;
    push(buf, Reg::Rax)
}

/// Generic binary i32 op consuming top two stack slots.
pub fn i32_bin(buf: &mut CodeBuffer, kind: Bin) -> Result<(), JitError> {
    pop(buf, Reg::Rcx)?;        // b
    pop(buf, Reg::Rax)?;        // a
    match kind {
        Bin::Add => alu::add_rr(buf, Reg::Rax, Reg::Rcx)?,
        Bin::Sub => alu::sub_rr(buf, Reg::Rax, Reg::Rcx)?,
        Bin::Mul => alu::imul_rr(buf, Reg::Rax, Reg::Rcx)?,
        Bin::And => alu::logic_rr(buf, Logic::And, Reg::Rax, Reg::Rcx)?,
        Bin::Or  => alu::logic_rr(buf, Logic::Or , Reg::Rax, Reg::Rcx)?,
        Bin::Xor => alu::logic_rr(buf, Logic::Xor, Reg::Rax, Reg::Rcx)?,
    }
    push(buf, Reg::Rax)
}
