//! Move helpers.  All operations are 64-bit unless explicitly narrowed.
use super::reg::{modrm, rex, Reg};
use crate::jit::codebuf::CodeBuffer;
use crate::jit::JitError;

/// `mov r64, imm64` — REX.W + B8+r io
pub fn mov_imm64(buf: &mut CodeBuffer, dst: Reg, imm: u64) -> Result<(), JitError> {
    buf.emit_u8(rex(true, 0, 0, dst.rex_b()))?;
    buf.emit_u8(0xB8 | dst.low())?;
    buf.emit_u64(imm)
}

/// `mov dst, src`  — 64-bit register move (REX.W + 89 /r).
pub fn mov_rr(buf: &mut CodeBuffer, dst: Reg, src: Reg) -> Result<(), JitError> {
    buf.emit_u8(rex(true, src.rex_b(), 0, dst.rex_b()))?;
    buf.emit_u8(0x89)?;
    buf.emit_u8(modrm(0b11, src.low(), dst.low()))
}

/// `mov dst, [base + disp32]`  — 64-bit load with 32-bit displacement.
pub fn mov_load64(buf: &mut CodeBuffer, dst: Reg, base: Reg, disp: i32) -> Result<(), JitError> {
    buf.emit_u8(rex(true, dst.rex_b(), 0, base.rex_b()))?;
    buf.emit_u8(0x8B)?;
    buf.emit_u8(modrm(0b10, dst.low(), base.low()))?;
    if base == Reg::Rsp { buf.emit_u8(0x24)?; }
    buf.emit_u32(disp as u32)
}

/// `mov [base + disp32], src` — 64-bit store with 32-bit displacement.
pub fn mov_store64(buf: &mut CodeBuffer, base: Reg, disp: i32, src: Reg) -> Result<(), JitError> {
    buf.emit_u8(rex(true, src.rex_b(), 0, base.rex_b()))?;
    buf.emit_u8(0x89)?;
    buf.emit_u8(modrm(0b10, src.low(), base.low()))?;
    if base == Reg::Rsp { buf.emit_u8(0x24)?; }
    buf.emit_u32(disp as u32)
}

/// `mov dst32, [base + disp32]` — 32-bit load (no REX.W).
pub fn mov_load32(buf: &mut CodeBuffer, dst: Reg, base: Reg, disp: i32) -> Result<(), JitError> {
    if dst.rex_b() != 0 || base.rex_b() != 0 {
        buf.emit_u8(rex(false, dst.rex_b(), 0, base.rex_b()))?;
    }
    buf.emit_u8(0x8B)?;
    buf.emit_u8(modrm(0b10, dst.low(), base.low()))?;
    if base == Reg::Rsp { buf.emit_u8(0x24)?; }
    buf.emit_u32(disp as u32)
}

/// `mov dst32, [base + index*scale]` — 32-bit indexed load via SIB.
///
/// `scale` is encoded as the log₂ (0=×1, 1=×2, 2=×4, 3=×8).  Used by
/// the JIT to emit Wasm `i32.load` as a single MMU access:
/// `[r15 + rax*1]`.
pub fn mov_load32_indexed(
    buf:   &mut CodeBuffer,
    dst:   Reg,
    base:  Reg,
    index: Reg,
    scale: u8,
) -> Result<(), JitError> {
    debug_assert!(scale <= 3);
    let r = dst.rex_b();
    let x = index.rex_b();
    let b = base.rex_b();
    if r != 0 || x != 0 || b != 0 {
        buf.emit_u8(rex(false, r, x, b))?;
    }
    buf.emit_u8(0x8B)?;
    buf.emit_u8(modrm(0b00, dst.low(), 0b100))?;     // Mod=00, RM=100 → SIB
    buf.emit_u8(((scale & 0b11) << 6) | (index.low() << 3) | base.low())?;
    Ok(())
}

/// `mov [base + index*scale], src32` — 32-bit indexed store via SIB.
pub fn mov_store32_indexed(
    buf:   &mut CodeBuffer,
    base:  Reg,
    index: Reg,
    scale: u8,
    src:   Reg,
) -> Result<(), JitError> {
    debug_assert!(scale <= 3);
    let r = src.rex_b();
    let x = index.rex_b();
    let b = base.rex_b();
    if r != 0 || x != 0 || b != 0 {
        buf.emit_u8(rex(false, r, x, b))?;
    }
    buf.emit_u8(0x89)?;
    buf.emit_u8(modrm(0b00, src.low(), 0b100))?;
    buf.emit_u8(((scale & 0b11) << 6) | (index.low() << 3) | base.low())?;
    Ok(())
}
