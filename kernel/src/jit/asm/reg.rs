//! x86_64 GPR encoding.
//!
//! 4-bit register field; the high bit goes to the REX prefix when set.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Reg {
    Rax = 0, Rcx = 1, Rdx = 2, Rbx = 3,
    Rsp = 4, Rbp = 5, Rsi = 6, Rdi = 7,
    R8  = 8, R9  = 9, R10 = 10, R11 = 11,
    R12 = 12, R13 = 13, R14 = 14, R15 = 15,
}

impl Reg {
    /// Low 3 bits — the modr/m field.
    #[inline] pub fn low(self) -> u8 { (self as u8) & 0b111 }
    /// High bit — gets folded into the REX prefix.
    #[inline] pub fn rex_b(self) -> u8 { ((self as u8) >> 3) & 1 }
}

/// Compute the REX prefix byte.  At minimum bit `0x40` is set (REX);
/// `W=1` selects 64-bit operand size; `R/X/B` extend register fields.
#[inline]
pub fn rex(w: bool, r: u8, x: u8, b: u8) -> u8 {
    0x40 | (if w { 0x08 } else { 0 }) | ((r & 1) << 2) | ((x & 1) << 1) | (b & 1)
}

/// Compose a Mod-R/M byte.
#[inline]
pub fn modrm(mode: u8, reg: u8, rm: u8) -> u8 {
    ((mode & 0b11) << 6) | ((reg & 0b111) << 3) | (rm & 0b111)
}
