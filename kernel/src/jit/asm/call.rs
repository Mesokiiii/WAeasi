//! Call / return + System-V argument shuffling.
//!
//! Wasm "host imports" call native helpers — we emit a standard 6-arg
//! System-V calling convention shim:
//!   rdi, rsi, rdx, rcx, r8, r9   ← integer args 1..6
//!   rax                            ← return.
use super::reg::{modrm, rex, Reg};
use crate::jit::codebuf::CodeBuffer;
use crate::jit::JitError;

/// `call rel32` — relative call, emits a placeholder displacement.
pub fn call_rel32(buf: &mut CodeBuffer, disp: i32) -> Result<usize, JitError> {
    buf.emit_u8(0xE8)?;
    let off = buf.len();
    buf.emit_u32(disp as u32)?;
    Ok(off)
}

/// `call rax` — indirect call through register.
pub fn call_indirect(buf: &mut CodeBuffer, target: Reg) -> Result<(), JitError> {
    if target.rex_b() != 0 { buf.emit_u8(rex(false, 0, 0, target.rex_b()))?; }
    buf.emit_u8(0xFF)?;
    buf.emit_u8(modrm(0b11, 2, target.low()))
}

/// `ret`
pub fn ret(buf: &mut CodeBuffer) -> Result<(), JitError> { buf.emit_u8(0xC3) }

/// `push reg`
pub fn push(buf: &mut CodeBuffer, reg: Reg) -> Result<(), JitError> {
    if reg.rex_b() != 0 { buf.emit_u8(rex(false, 0, 0, reg.rex_b()))?; }
    buf.emit_u8(0x50 | reg.low())
}

/// `pop reg`
pub fn pop(buf: &mut CodeBuffer, reg: Reg) -> Result<(), JitError> {
    if reg.rex_b() != 0 { buf.emit_u8(rex(false, 0, 0, reg.rex_b()))?; }
    buf.emit_u8(0x58 | reg.low())
}
