//! i32 numeric ops — every helper compiles to 1-2 native instructions.
//!
//! `#[inline(always)]` is mandatory: at trillions of ops/sec a single
//! call-site through a non-inlined helper costs measurable cycles.
use super::value::Cell;

#[inline(always)] pub fn add (a: i32, b: i32) -> Cell { Cell::from_i32(a.wrapping_add(b)) }
#[inline(always)] pub fn sub (a: i32, b: i32) -> Cell { Cell::from_i32(a.wrapping_sub(b)) }
#[inline(always)] pub fn mul (a: i32, b: i32) -> Cell { Cell::from_i32(a.wrapping_mul(b)) }

#[inline] pub fn div_s(a: i32, b: i32) -> Result<Cell, ()> {
    if b == 0 || (a == i32::MIN && b == -1) { return Err(()); }
    Ok(Cell::from_i32(a / b))
}
#[inline] pub fn div_u(a: u32, b: u32) -> Result<Cell, ()> {
    if b == 0 { return Err(()); }
    Ok(Cell::from_u32(a / b))
}
#[inline] pub fn rem_s(a: i32, b: i32) -> Result<Cell, ()> {
    if b == 0 { return Err(()); }
    Ok(Cell::from_i32(a.wrapping_rem(b)))
}
#[inline] pub fn rem_u(a: u32, b: u32) -> Result<Cell, ()> {
    if b == 0 { return Err(()); }
    Ok(Cell::from_u32(a % b))
}

#[inline(always)] pub fn and(a: i32, b: i32) -> Cell { Cell::from_i32(a & b) }
#[inline(always)] pub fn or (a: i32, b: i32) -> Cell { Cell::from_i32(a | b) }
#[inline(always)] pub fn xor(a: i32, b: i32) -> Cell { Cell::from_i32(a ^ b) }

#[inline(always)] pub fn shl  (a: i32, b: i32) -> Cell { Cell::from_i32(a.wrapping_shl((b & 31) as u32)) }
#[inline(always)] pub fn shr_s(a: i32, b: i32) -> Cell { Cell::from_i32(a.wrapping_shr((b & 31) as u32)) }
#[inline(always)] pub fn shr_u(a: i32, b: i32) -> Cell {
    Cell::from_u32((a as u32).wrapping_shr((b & 31) as u32))
}
#[inline(always)] pub fn rotl(a: i32, b: i32) -> Cell { Cell::from_i32(a.rotate_left ((b & 31) as u32)) }
#[inline(always)] pub fn rotr(a: i32, b: i32) -> Cell { Cell::from_i32(a.rotate_right((b & 31) as u32)) }

#[inline(always)] pub fn eqz(a: i32) -> Cell { Cell::from_i32((a == 0) as i32) }
#[inline(always)] pub fn eq (a: i32, b: i32) -> Cell { Cell::from_i32((a == b) as i32) }
#[inline(always)] pub fn ne (a: i32, b: i32) -> Cell { Cell::from_i32((a != b) as i32) }
#[inline(always)] pub fn lt_s(a: i32, b: i32) -> Cell { Cell::from_i32((a <  b) as i32) }
#[inline(always)] pub fn lt_u(a: u32, b: u32) -> Cell { Cell::from_i32((a <  b) as i32) }
#[inline(always)] pub fn gt_s(a: i32, b: i32) -> Cell { Cell::from_i32((a >  b) as i32) }
#[inline(always)] pub fn gt_u(a: u32, b: u32) -> Cell { Cell::from_i32((a >  b) as i32) }
#[inline(always)] pub fn le_s(a: i32, b: i32) -> Cell { Cell::from_i32((a <= b) as i32) }
#[inline(always)] pub fn le_u(a: u32, b: u32) -> Cell { Cell::from_i32((a <= b) as i32) }
#[inline(always)] pub fn ge_s(a: i32, b: i32) -> Cell { Cell::from_i32((a >= b) as i32) }
#[inline(always)] pub fn ge_u(a: u32, b: u32) -> Cell { Cell::from_i32((a >= b) as i32) }
