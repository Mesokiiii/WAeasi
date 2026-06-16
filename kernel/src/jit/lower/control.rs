//! Control-flow lowering with patched relocations.
//!
//! Stage 3 supports two function-scope labels:
//!   * `end_label`  — beginning of the epilogue (`br`/`return` jump here).
//!   * `trap_label` — fall-through to the trap shim (set rax=-1, jmp end).
//!
//! Stage 4 adds nested labels for `block`/`loop`/`if`.
use crate::jit::asm::branch::{self, cc};
use crate::jit::asm::call::pop;
use crate::jit::asm::mov;
use crate::jit::asm::reg::Reg;
use crate::jit::codebuf::CodeBuffer;
use crate::jit::labels::{LabelId, LabelTable};
use crate::jit::JitError;
use crate::wasm::parser::leb_u32;

pub fn br(
    buf:    &mut CodeBuffer,
    bytes:  &[u8],
    p:      &mut usize,
    labels: &mut LabelTable,
    end:    LabelId,
) -> Result<(), JitError> {
    let _depth = leb_u32(bytes, p).map_err(|_| JitError::Truncated)?;
    let patch = branch::jmp_rel32(buf, 0)?;
    labels.add_pending(end, patch, buf.len());
    Ok(())
}

pub fn br_if(
    buf:    &mut CodeBuffer,
    bytes:  &[u8],
    p:      &mut usize,
    labels: &mut LabelTable,
    end:    LabelId,
) -> Result<(), JitError> {
    let _depth = leb_u32(bytes, p).map_err(|_| JitError::Truncated)?;
    pop(buf, Reg::Rax)?;
    branch::test_rr(buf, Reg::Rax, Reg::Rax)?;
    let patch = branch::jcc_rel32(buf, cc::JNE, 0)?;
    labels.add_pending(end, patch, buf.len());
    Ok(())
}

/// `unreachable` — jump to the trap label (which sets rax = u64::MAX
/// and falls through to the epilogue).
pub fn trap(
    buf:    &mut CodeBuffer,
    labels: &mut LabelTable,
    trap_l: LabelId,
) -> Result<(), JitError> {
    let patch = branch::jmp_rel32(buf, 0)?;
    labels.add_pending(trap_l, patch, buf.len());
    Ok(())
}

/// Emit the trap shim itself: `mov rax, -1; jmp end`.
pub fn emit_trap_shim(
    buf:    &mut CodeBuffer,
    labels: &mut LabelTable,
    end:    LabelId,
) -> Result<(), JitError> {
    mov::mov_imm64(buf, Reg::Rax, u64::MAX)?;
    let patch = branch::jmp_rel32(buf, 0)?;
    labels.add_pending(end, patch, buf.len());
    Ok(())
}
