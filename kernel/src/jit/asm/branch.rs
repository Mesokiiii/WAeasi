//! Branch / compare emit helpers.
use super::reg::{modrm, rex, Reg};
use crate::jit::codebuf::CodeBuffer;
use crate::jit::JitError;

/// `cmp dst, src`  (REX.W + 39 /r)
pub fn cmp_rr(buf: &mut CodeBuffer, dst: Reg, src: Reg) -> Result<(), JitError> {
    buf.emit_u8(rex(true, src.rex_b(), 0, dst.rex_b()))?;
    buf.emit_u8(0x39)?;
    buf.emit_u8(modrm(0b11, src.low(), dst.low()))
}

/// `test dst, src` (REX.W + 85 /r) — sets ZF/SF without touching dst.
pub fn test_rr(buf: &mut CodeBuffer, dst: Reg, src: Reg) -> Result<(), JitError> {
    buf.emit_u8(rex(true, src.rex_b(), 0, dst.rex_b()))?;
    buf.emit_u8(0x85)?;
    buf.emit_u8(modrm(0b11, src.low(), dst.low()))
}

/// Unconditional 32-bit relative jump.  Returns the offset of the
/// `disp32` so the caller can patch it after the target is known.
pub fn jmp_rel32(buf: &mut CodeBuffer, disp: i32) -> Result<usize, JitError> {
    buf.emit_u8(0xE9)?;
    let off = buf.len();
    buf.emit_u32(disp as u32)?;
    Ok(off)
}

/// Conditional 32-bit relative jump (`jcc`).  `cond` is the 4-bit
/// condition (0x0=jo, 0x4=je, 0xC=jl, 0xD=jge ...).
pub fn jcc_rel32(buf: &mut CodeBuffer, cond: u8, disp: i32) -> Result<usize, JitError> {
    buf.emit_u8(0x0F)?;
    buf.emit_u8(0x80 | (cond & 0x0F))?;
    let off = buf.len();
    buf.emit_u32(disp as u32)?;
    Ok(off)
}

pub mod cc {
    pub const JE:  u8 = 0x4;
    pub const JNE: u8 = 0x5;
    pub const JL:  u8 = 0xC;
    pub const JGE: u8 = 0xD;
    pub const JLE: u8 = 0xE;
    pub const JG:  u8 = 0xF;
    pub const JB:  u8 = 0x2;
    pub const JAE: u8 = 0x3;
    pub const JBE: u8 = 0x6;
    pub const JA:  u8 = 0x7;
}
