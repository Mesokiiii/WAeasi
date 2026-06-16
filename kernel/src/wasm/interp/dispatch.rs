//! Main interpreter dispatch loop.
//!
//! Two entry points:
//!   * `run`              — convenience wrapper (allocates a fresh stack).
//!     Useful for one-shot tests and the `Instance` cold path.
//!   * `run_with_stack`   — performance entry: caller-provided
//!     `ValueStack` is **reused across invocations**.  At trillions of
//!     Wasm calls per second this eliminates the per-call 32 KiB heap
//!     churn that the dedicated allocator path otherwise pays.
//!
//! Hot-path principles:
//!   * `Cell` is 8 bytes — values move via single `mov`.
//!   * `ValueStack` uses `get_unchecked` (validator-proven bounds).
//!   * Cold paths (`Trap`) are `#[cold]`.
use alloc::vec::Vec;

use super::control;
use super::frame::{CallStack, Frame};
use super::memory as mem_ops;
use super::numeric as n;
use super::opcode::*;
use super::stack::ValueStack;
use super::value::Cell;
use crate::memory::linear_mem::LinearMemory;
use crate::wasm::parser::leb_i32 as r_i32;
use crate::wasm::parser::leb_i64 as r_i64;
use crate::wasm::parser::leb_u32 as r_u32;
use crate::wasm::trap::Trap;

const STACK_RESERVE: usize = 4096;

/// One-shot run — allocates a fresh stack.  Convenient but **not** the
/// scaled hot path.
pub fn run<'a>(
    code: &'a [u8],
    params: Vec<Cell>,
    mem:    Option<&LinearMemory>,
) -> Result<Vec<Cell>, Trap> {
    let mut stack = ValueStack::with_capacity(STACK_RESERVE);
    run_with_stack(&mut stack, code, params, mem)
}

/// Performance entry — caller owns the operand stack and reuses it
/// across calls.  Stack is `truncate(0)`'d on entry; capacity is left.
pub fn run_with_stack<'a>(
    stack:  &mut ValueStack,
    code:   &'a [u8],
    params: Vec<Cell>,
    mem:    Option<&LinearMemory>,
) -> Result<Vec<Cell>, Trap> {
    stack.truncate(0);
    let mut calls = CallStack::new();
    calls.push(Frame { code, pc: 0, locals: params, stack_base: 0, n_results: 0 });

    while let Some(frame) = calls.top() {
        if frame.pc >= frame.code.len() { break; }
        let op = unsafe { *frame.code.get_unchecked(frame.pc) };
        frame.pc += 1;

        match op {
            UNREACHABLE => return cold_trap(Trap::Unreachable),
            NOP         => {}
            END         => { let _ = calls.pop(); }
            DROP        => { let _ = stack.pop(); }
            RETURN      => { let _ = calls.pop(); }

            LOCAL_GET => {
                let idx = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)? as usize;
                stack.push(unsafe { *frame.locals.get_unchecked(idx) });
            }
            LOCAL_SET => {
                let idx = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)? as usize;
                let v = stack.pop();
                unsafe { *frame.locals.get_unchecked_mut(idx) = v; }
            }
            LOCAL_TEE => {
                let idx = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)? as usize;
                let v = stack.peek(0);
                unsafe { *frame.locals.get_unchecked_mut(idx) = v; }
            }

            I32_CONST => {
                let v = r_i32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)?;
                stack.push(Cell::from_i32(v));
            }
            I64_CONST => {
                let v = r_i64(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)?;
                stack.push(Cell::from_i64(v));
            }

            I32_ADD => bin_i32(stack, n::add),
            I32_SUB => bin_i32(stack, n::sub),
            I32_MUL => bin_i32(stack, n::mul),
            I32_AND => bin_i32(stack, n::and),
            I32_OR  => bin_i32(stack, n::or ),
            I32_XOR => bin_i32(stack, n::xor),
            I32_SHL   => bin_i32(stack, n::shl),
            I32_SHR_S => bin_i32(stack, n::shr_s),
            I32_SHR_U => bin_i32(stack, n::shr_u),
            I32_ROTL  => bin_i32(stack, n::rotl),
            I32_ROTR  => bin_i32(stack, n::rotr),

            I32_EQ  => bin_i32(stack, n::eq ),
            I32_NE  => bin_i32(stack, n::ne ),
            I32_LT_S=> bin_i32(stack, n::lt_s),
            I32_GT_S=> bin_i32(stack, n::gt_s),
            I32_LE_S=> bin_i32(stack, n::le_s),
            I32_GE_S=> bin_i32(stack, n::ge_s),
            I32_LT_U=> bin_u32(stack, n::lt_u),
            I32_GT_U=> bin_u32(stack, n::gt_u),
            I32_LE_U=> bin_u32(stack, n::le_u),
            I32_GE_U=> bin_u32(stack, n::ge_u),
            I32_EQZ => { let a = stack.pop_i32(); stack.push(n::eqz(a)); }

            I32_DIV_S => {
                let b = stack.pop_i32(); let a = stack.pop_i32();
                stack.push(n::div_s(a,b).map_err(|_| Trap::DivisionByZero)?);
            }
            I32_DIV_U => {
                let b = stack.pop_u32(); let a = stack.pop_u32();
                stack.push(n::div_u(a,b).map_err(|_| Trap::DivisionByZero)?);
            }

            I32_LOAD => {
                let _a = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)?;
                let off = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)?;
                let base = stack.pop_u32();
                let m = mem.ok_or(Trap::OutOfBounds)?;
                stack.push(Cell::from_i32(mem_ops::i32_load(m, base, off)?));
            }
            I32_STORE => {
                let _a = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)?;
                let off = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)?;
                let val = stack.pop_i32();
                let base = stack.pop_u32();
                let m = mem.ok_or(Trap::OutOfBounds)?;
                mem_ops::i32_store(m, base, off, val)?;
            }

            BR => {
                let _d = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)?;
                frame.pc = control::skip_to_end(frame.code, frame.pc);
            }
            BR_IF => {
                let _d = r_u32(frame.code, &mut frame.pc).map_err(|_| Trap::HostError)?;
                let cond = stack.pop_i32();
                if cond != 0 { frame.pc = control::skip_to_end(frame.code, frame.pc); }
            }

            _ => return cold_trap(Trap::HostError),
        }
    }

    let out: Vec<Cell> = stack.as_slice().to_vec();
    Ok(out)
}

#[inline(always)]
fn bin_i32(s: &mut ValueStack, f: fn(i32, i32) -> Cell) {
    let b = s.pop_i32(); let a = s.pop_i32(); s.push(f(a, b));
}
#[inline(always)]
fn bin_u32(s: &mut ValueStack, f: fn(u32, u32) -> Cell) {
    let b = s.pop_u32(); let a = s.pop_u32(); s.push(f(a, b));
}

#[cold]
fn cold_trap<T>(t: Trap) -> Result<T, Trap> { Err(t) }
