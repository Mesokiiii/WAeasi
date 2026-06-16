//! Control-flow primitives for the interpreter.
//!
//! `block`/`loop`/`if`/`br`/`br_if`/`return` all share the same engine:
//! they manipulate the call stack + value stack to skip forward to a
//! corresponding `end` (for `block`/`if`) or back to the beginning of
//! the loop (for `loop`).
//!
//! Stage 2 implements skip-forward via a simple linear scan.  Stage 3
//! will pre-compute jump tables during validation so this becomes O(1).
use super::stack::ValueStack;
use crate::wasm::interp::opcode::*;

/// Skip forward in `code` from `pc` until we balance to the matching
/// `END`, accounting for nested `block`/`loop`/`if` frames.
pub fn skip_to_end(code: &[u8], mut pc: usize) -> usize {
    let mut depth = 1;
    while pc < code.len() && depth > 0 {
        let op = code[pc]; pc += 1;
        match op {
            BLOCK | LOOP | IF_ => { depth += 1; pc = skip_blocktype(code, pc); }
            END => depth -= 1,
            _ => pc = skip_immediates(code, pc, op),
        }
    }
    pc
}

/// Move past the block-type byte (single-byte form for MVP).
fn skip_blocktype(_code: &[u8], pc: usize) -> usize { pc + 1 }

/// Conservative skip — every immediate kind we currently emit is a
/// signed/unsigned LEB128.  Stage 3 replaces with a precomputed table.
fn skip_immediates(code: &[u8], mut pc: usize, op: u8) -> usize {
    match op {
        BR | BR_IF | LOCAL_GET | LOCAL_SET | LOCAL_TEE | CALL
        | I32_CONST | I64_CONST => {
            while pc < code.len() && code[pc] & 0x80 != 0 { pc += 1; }
            pc + 1
        }
        I32_LOAD | I32_STORE => {
            // Two LEBs: align + offset.
            for _ in 0..2 {
                while pc < code.len() && code[pc] & 0x80 != 0 { pc += 1; }
                pc += 1;
            }
            pc
        }
        _ => pc,
    }
}

/// Drop top-of-stack `n` values.
#[inline]
pub fn drop_n(stack: &mut ValueStack, n: usize) {
    let new_len = stack.len().saturating_sub(n);
    stack.truncate(new_len);
}
