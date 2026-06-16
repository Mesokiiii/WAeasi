//! Wasm runtime value — packed into a single 8-byte raw cell.
//!
//! The validator proves every operand's type **before** the interpreter
//! runs, so carrying a runtime tag is wasted memory.  At trillions of
//! processes / billions of stack pushes per second, halving the cell
//! size halves cache pressure and L2/L3 traffic.
//!
//! Reading the wrong type is a *bug*, not a recoverable error — the
//! validator should have caught it.  Helpers are debug-asserted, optimized
//! away in release.
use core::mem::transmute;

/// 8-byte raw cell.  All Wasm types fit:  i32 (low 32), i64, f32 bits,
/// f64 bits, ref handles.
#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct Cell(pub u64);

impl Cell {
    #[inline(always)] pub const fn from_i32(v: i32) -> Self { Self(v as u32 as u64) }
    #[inline(always)] pub const fn from_i64(v: i64) -> Self { Self(v as u64) }
    #[inline(always)] pub const fn from_u32(v: u32) -> Self { Self(v as u64) }
    #[inline(always)] pub const fn from_u64(v: u64) -> Self { Self(v) }
    #[inline(always)] pub fn from_f32(v: f32)  -> Self { Self(v.to_bits() as u64) }
    #[inline(always)] pub fn from_f64(v: f64)  -> Self { Self(v.to_bits()) }

    #[inline(always)] pub const fn as_i32(self) -> i32 { self.0 as i32 }
    #[inline(always)] pub const fn as_i64(self) -> i64 { self.0 as i64 }
    #[inline(always)] pub const fn as_u32(self) -> u32 { self.0 as u32 }
    #[inline(always)] pub const fn as_u64(self) -> u64 { self.0 }
    #[inline(always)] pub fn as_f32(self) -> f32 { f32::from_bits(self.0 as u32) }
    #[inline(always)] pub fn as_f64(self) -> f64 { f64::from_bits(self.0) }
}

/// Compatibility alias used by call sites that haven't migrated.
pub type Value = Cell;

/// Static check — must be exactly 8 bytes for cache math to hold.
const _: () = assert!(core::mem::size_of::<Cell>() == 8);
const _: () = assert!(core::mem::align_of::<Cell>() == 8);

/// Reinterpret a `[Cell]` as `[u64]` for vector copies (memcpy fast path).
#[inline(always)]
pub fn as_u64_slice(s: &[Cell]) -> &[u64] {
    unsafe { transmute(s) }
}
