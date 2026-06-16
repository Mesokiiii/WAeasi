//! ALU emit helpers.  All ops produce 64-bit forms (REX.W).
use super::reg::{modrm, rex, Reg};
use crate::jit::codebuf::CodeBuffer;
use crate::jit::JitError;

/// `add dst, src`
pub fn add_rr(buf: &mut CodeBuffer, dst: Reg, src: Reg) -> Result<(), JitError> {
    buf.emit_u8(rex(true, src.rex_b(), 0, dst.rex_b()))?;
    buf.emit_u8(0x01)?;
    buf.emit_u8(modrm(0b11, src.low(), dst.low()))
}

/// `sub dst, src`
pub fn sub_rr(buf: &mut CodeBuffer, dst: Reg, src: Reg) -> Result<(), JitError> {
    buf.emit_u8(rex(true, src.rex_b(), 0, dst.rex_b()))?;
    buf.emit_u8(0x29)?;
    buf.emit_u8(modrm(0b11, src.low(), dst.low()))
}

/// `imul dst, src`  (REX.W + 0F AF /r)
pub fn imul_rr(buf: &mut CodeBuffer, dst: Reg, src: Reg) -> Result<(), JitError> {
    buf.emit_u8(rex(true, dst.rex_b(), 0, src.rex_b()))?;
    buf.emit_u8(0x0F)?;
    buf.emit_u8(0xAF)?;
    buf.emit_u8(modrm(0b11, dst.low(), src.low()))
}

/// `and / or / xor dst, src` — opcode picked from `kind`.
pub fn logic_rr(buf: &mut CodeBuffer, kind: Logic, dst: Reg, src: Reg) -> Result<(), JitError> {
    let op = match kind { Logic::And => 0x21, Logic::Or => 0x09, Logic::Xor => 0x31 };
    buf.emit_u8(rex(true, src.rex_b(), 0, dst.rex_b()))?;
    buf.emit_u8(op)?;
    buf.emit_u8(modrm(0b11, src.low(), dst.low()))
}

#[derive(Copy, Clone, Debug)] pub enum Logic { And, Or, Xor }

/// `add dst, imm32`  (REX.W + 81 /0 id)
pub fn add_imm32(buf: &mut CodeBuffer, dst: Reg, imm: i32) -> Result<(), JitError> {
    buf.emit_u8(rex(true, 0, 0, dst.rex_b()))?;
    buf.emit_u8(0x81)?;
    buf.emit_u8(modrm(0b11, 0, dst.low()))?;
    buf.emit_u32(imm as u32)
}
