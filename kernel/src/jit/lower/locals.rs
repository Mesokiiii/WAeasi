//! `local.get/set/tee` lowering.
//!
//! Locals live at `[rbp - 8*(idx+1)]`.  `set`/`tee` write 64 bits even
//! though Wasm i32 is 32-bit — the upper bits are zero-extended, which
//! matches Wasm's i32 semantics on a 64-bit slot.
use crate::jit::asm::call::{pop, push};
use crate::jit::asm::mov;
use crate::jit::asm::reg::Reg;
use crate::jit::codebuf::CodeBuffer;
use crate::jit::JitError;
use crate::wasm::parser::leb_u32;

#[inline]
fn slot_offset(idx: u32) -> i32 {
    -(((idx + 1) as i32) * 8)
}

pub fn get(buf: &mut CodeBuffer, bytes: &[u8], p: &mut usize) -> Result<(), JitError> {
    let idx = leb_u32(bytes, p).map_err(|_| JitError::Truncated)?;
    mov::mov_load64(buf, Reg::Rax, Reg::Rbp, slot_offset(idx))?;
    push(buf, Reg::Rax)
}

pub fn set(buf: &mut CodeBuffer, bytes: &[u8], p: &mut usize) -> Result<(), JitError> {
    let idx = leb_u32(bytes, p).map_err(|_| JitError::Truncated)?;
    pop(buf, Reg::Rax)?;
    mov::mov_store64(buf, Reg::Rbp, slot_offset(idx), Reg::Rax)
}

pub fn tee(buf: &mut CodeBuffer, bytes: &[u8], p: &mut usize) -> Result<(), JitError> {
    let idx = leb_u32(bytes, p).map_err(|_| JitError::Truncated)?;
    pop(buf, Reg::Rax)?;
    mov::mov_store64(buf, Reg::Rbp, slot_offset(idx), Reg::Rax)?;
    push(buf, Reg::Rax)
}
