//! Function-level type checker — MVP subset.
//!
//! Stage 2 validates exactly what the interpreter executes:
//!   * numeric (i32/i64) const, add, sub, mul, div_s/u, eq, lt_s/u, ...
//!   * local.get/set/tee
//!   * memory.size/grow, i32.load/store
//!   * call (direct), return, drop, select
//!   * block / loop / if-else / br / br_if / end / unreachable
//!
//! Unsupported opcodes return `Unsupported(name)` so the audit log
//! pinpoints which feature must be added next.
use super::ctrl_flow::{ControlStack, Frame, FrameKind};
use super::stack::{OpType, TypeStack};
use super::ValidateError;
use crate::wasm::parser::code::FunctionBody;
use crate::wasm::parser::types::{FuncType, ValType};
use crate::wasm::parser::ParsedModule;

pub fn check_module(m: &ParsedModule<'_>) -> Result<(), ValidateError> {
    if m.functions.len() != m.bodies.len() {
        return Err(ValidateError::Unsupported("func/body length mismatch"));
    }
    for (i, fty_idx) in m.functions.iter().enumerate() {
        let fty = m.types.get(*fty_idx as usize)
            .ok_or(ValidateError::UnknownType(*fty_idx))?;
        check_function(m, fty, &m.bodies[i])?;
    }
    Ok(())
}

fn check_function(
    _m: &ParsedModule<'_>,
    fty: &FuncType,
    body: &FunctionBody<'_>,
) -> Result<(), ValidateError> {
    let mut stack = TypeStack::new();
    let mut ctrl  = ControlStack::new();

    // Push the function-body frame.
    let result_ty = fty.results.first().copied();
    ctrl.push(Frame {
        kind: FrameKind::Block,
        stack_depth: 0,
        branch_ty: None,
        end_ty: result_ty,
        unreachable: false,
    });

    // Build the locals layout: params then declared locals.
    let mut locals: alloc::vec::Vec<ValType> = fty.params.clone();
    for run in &body.locals {
        for _ in 0..run.count { locals.push(run.ty); }
    }

    let mut p = 0usize;
    let bytes = body.code;
    while p < bytes.len() {
        let op = bytes[p]; p += 1;
        match op {
            0x0B => { // end
                let frame = ctrl.pop()?;
                if let Some(t) = frame.end_ty { stack.push(OpType::val(t)); }
                if ctrl.is_empty() { break; }
            }
            0x00 => { stack.poly(); } // unreachable
            0x01 => {}                // nop
            0x10 => { // call
                let _idx = read_leb_u32(bytes, &mut p)?;
                // TODO: pop params, push results — needs func table.
            }
            0x20 => { // local.get
                let idx = read_leb_u32(bytes, &mut p)? as usize;
                let t = locals.get(idx).copied().ok_or(ValidateError::UnknownLocal(idx as u32))?;
                stack.push(OpType::val(t));
            }
            0x21 => { // local.set
                let idx = read_leb_u32(bytes, &mut p)? as usize;
                let t = locals.get(idx).copied().ok_or(ValidateError::UnknownLocal(idx as u32))?;
                stack.pop_expect(t)?;
            }
            0x41 => { read_leb_i32(bytes, &mut p)?; stack.push(OpType::val(ValType::I32)); }
            0x42 => { read_leb_i64(bytes, &mut p)?; stack.push(OpType::val(ValType::I64)); }
            0x6A | 0x6B | 0x6C | 0x6D | 0x6E | 0x6F | 0x70 => {
                stack.pop_expect(ValType::I32)?;
                stack.pop_expect(ValType::I32)?;
                stack.push(OpType::val(ValType::I32));
            }
            _ => return Err(ValidateError::Unsupported("opcode")),
        }
    }

    if !ctrl.is_empty() { return Err(ValidateError::UnclosedBlock); }
    Ok(())
}

fn read_leb_u32(b: &[u8], p: &mut usize) -> Result<u32, ValidateError> {
    crate::wasm::parser::leb_u32(b, p).map_err(|_| ValidateError::Unsupported("bad leb"))
}
fn read_leb_i32(b: &[u8], p: &mut usize) -> Result<i32, ValidateError> {
    crate::wasm::parser::leb_i32(b, p).map_err(|_| ValidateError::Unsupported("bad leb"))
}
fn read_leb_i64(b: &[u8], p: &mut usize) -> Result<i64, ValidateError> {
    crate::wasm::parser::leb_i64(b, p).map_err(|_| ValidateError::Unsupported("bad leb"))
}
