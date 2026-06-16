//! Bounds-checked Wasm linear-memory access.
//!
//! Hot path:
//!   1. Single combined check `addr+4 <= len` (overflow-safe via `>` only).
//!   2. `array_index_nospec` — branch-free clamp + lfence (Spectre v1).
//!   3. Direct unaligned load via `read_unaligned` / `write_unaligned`.
//!
//! Validator already proved the operand types — no `Cell` tag check.
use crate::memory::linear_mem::LinearMemory;
use crate::security::speculation::array_index_nospec;
use crate::wasm::trap::Trap;

#[inline(always)]
pub fn i32_load(mem: &LinearMemory, base: u32, offset: u32) -> Result<i32, Trap> {
    let addr = (base as usize).wrapping_add(offset as usize);
    let len  = mem.len();
    // single combined bound check — no checked_add (overflow folds into the >).
    if addr.checked_add(4).map_or(true, |e| e > len) { return cold_oob(); }
    let safe = array_index_nospec(addr, len.saturating_sub(3));
    let p = unsafe { mem.as_slice_mut().as_ptr().add(safe) as *const i32 };
    Ok(unsafe { core::ptr::read_unaligned(p) })
}

#[inline(always)]
pub fn i32_store(mem: &LinearMemory, base: u32, offset: u32, val: i32) -> Result<(), Trap> {
    let addr = (base as usize).wrapping_add(offset as usize);
    let len  = mem.len();
    if addr.checked_add(4).map_or(true, |e| e > len) { return cold_oob(); }
    let safe = array_index_nospec(addr, len.saturating_sub(3));
    let p = unsafe { mem.as_slice_mut().as_mut_ptr().add(safe) as *mut i32 };
    unsafe { core::ptr::write_unaligned(p, val) };
    Ok(())
}

#[inline(always)]
pub fn memory_size_pages(mem: &LinearMemory) -> i32 { mem.pages as i32 }

#[cold]
fn cold_oob<T>() -> Result<T, Trap> { Err(Trap::OutOfBounds) }
