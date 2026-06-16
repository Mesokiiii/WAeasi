//! Single-pass Wasm-to-native lowering.
//!
//! Per-function pipeline:
//!   1. Allocate `end_label` and `trap_label`.
//!   2. Emit prologue + body, recording relocations into `LabelTable`.
//!   3. Emit the trap shim, bind `trap_label` to its start.
//!   4. Bind `end_label` to the epilogue start.
//!   5. Patch every relocation; verify no dangling labels remain.
//!   6. `freeze()` is the caller's responsibility (so multi-function
//!      compiles can share a buffer).
pub mod control;
pub mod locals;
pub mod memory;
pub mod numeric;

use crate::wasm::interp::opcode::*;
use crate::wasm::parser::code::FunctionBody;

use super::asm::call::pop;
use super::asm::prologue;
use super::asm::reg::Reg;
use super::codebuf::CodeBuffer;
use super::labels::LabelTable;
use super::JitError;

pub fn compile_function<'a>(
    buf:        &mut CodeBuffer,
    body:       &FunctionBody<'a>,
    n_params:   u32,
    n_local_run:u32,
) -> Result<usize, JitError> {
    let entry = buf.len();
    let mut labels = LabelTable::new();
    let end_label  = labels.new_label();
    let trap_label = labels.new_label();

    let n_locals = n_params + n_local_run;
    prologue::prologue(buf, n_locals)?;

    let mut p = 0;
    let bytes = body.code;
    while p < bytes.len() {
        let op = bytes[p]; p += 1;
        match op {
            END         => break,
            NOP         => {}
            DROP        => pop(buf, Reg::Rax)?,
            I32_CONST   => numeric::i32_const(buf, bytes, &mut p)?,
            I32_ADD     => numeric::i32_bin(buf, numeric::Bin::Add)?,
            I32_SUB     => numeric::i32_bin(buf, numeric::Bin::Sub)?,
            I32_MUL     => numeric::i32_bin(buf, numeric::Bin::Mul)?,
            I32_AND     => numeric::i32_bin(buf, numeric::Bin::And)?,
            I32_OR      => numeric::i32_bin(buf, numeric::Bin::Or )?,
            I32_XOR     => numeric::i32_bin(buf, numeric::Bin::Xor)?,
            LOCAL_GET   => locals::get(buf, bytes, &mut p)?,
            LOCAL_SET   => locals::set(buf, bytes, &mut p)?,
            LOCAL_TEE   => locals::tee(buf, bytes, &mut p)?,
            I32_LOAD    => memory::i32_load (buf, bytes, &mut p, &mut labels, trap_label)?,
            I32_STORE   => memory::i32_store(buf, bytes, &mut p, &mut labels, trap_label)?,
            BR          => control::br   (buf, bytes, &mut p, &mut labels, end_label)?,
            BR_IF       => control::br_if(buf, bytes, &mut p, &mut labels, end_label)?,
            UNREACHABLE => control::trap (buf, &mut labels, trap_label)?,
            _ => return Err(JitError::Unsupported("opcode")),
        }
    }

    // 1. Trap shim — bind label to its first byte, then emit it.
    labels.bind(buf, trap_label, buf.len())?;
    control::emit_trap_shim(buf, &mut labels, end_label)?;

    // 2. End label = beginning of epilogue.
    labels.bind(buf, end_label, buf.len())?;
    prologue::epilogue(buf)?;

    labels.finalize()?;
    Ok(entry)
}
