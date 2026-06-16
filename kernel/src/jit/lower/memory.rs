//! Memory load/store lowering with **inline bounds check** and patched
//! OOB jumps to a function-scope trap label.
//!
//! Pseudo-asm for `i32.load offset`:
//! ```text
//!   pop rax                          ; base (u32)
//!   add rax, offset                  ; if offset != 0
//!   cmp rax, rsi                     ; rsi holds memory length
//!   jae <trap_label>                 ; unsigned compare; relocation
//!   mov eax, [r15 + rax*1]           ; SIB-indexed load
//!   push rax
//! ```
//!
//! The `jae` displacement is a patched relocation — the caller (lower/mod)
//! binds `trap_label` after the function body and `LabelTable::bind`
//! back-patches every jump.
use crate::jit::asm::alu;
use crate::jit::asm::branch::{self, cc};
use crate::jit::asm::call::{pop, push};
use crate::jit::asm::mov;
use crate::jit::asm::reg::Reg;
use crate::jit::codebuf::CodeBuffer;
use crate::jit::labels::{LabelId, LabelTable};
use crate::jit::JitError;
use crate::wasm::parser::leb_u32;

pub fn i32_load(
    buf:    &mut CodeBuffer,
    bytes:  &[u8],
    p:      &mut usize,
    labels: &mut LabelTable,
    trap:   LabelId,
) -> Result<(), JitError> {
    let _align = leb_u32(bytes, p).map_err(|_| JitError::Truncated)?;
    let offset = leb_u32(bytes, p).map_err(|_| JitError::Truncated)? as i32;

    pop(buf, Reg::Rax)?;
    if offset != 0 { alu::add_imm32(buf, Reg::Rax, offset)?; }
    branch::cmp_rr(buf, Reg::Rax, Reg::Rsi)?;

    // Pending relocation: jae trap.
    let patch = branch::jcc_rel32(buf, cc::JAE, 0)?;
    labels.add_pending(trap, patch, buf.len());

    mov::mov_load32_indexed(buf, Reg::Rax, Reg::R15, Reg::Rax, 0)?;
    push(buf, Reg::Rax)
}

pub fn i32_store(
    buf:    &mut CodeBuffer,
    bytes:  &[u8],
    p:      &mut usize,
    labels: &mut LabelTable,
    trap:   LabelId,
) -> Result<(), JitError> {
    let _align = leb_u32(bytes, p).map_err(|_| JitError::Truncated)?;
    let offset = leb_u32(bytes, p).map_err(|_| JitError::Truncated)? as i32;

    pop(buf, Reg::Rcx)?;     // value
    pop(buf, Reg::Rax)?;     // base
    if offset != 0 { alu::add_imm32(buf, Reg::Rax, offset)?; }
    branch::cmp_rr(buf, Reg::Rax, Reg::Rsi)?;

    let patch = branch::jcc_rel32(buf, cc::JAE, 0)?;
    labels.add_pending(trap, patch, buf.len());

    mov::mov_store32_indexed(buf, Reg::R15, Reg::Rax, 0, Reg::Rcx)
}
