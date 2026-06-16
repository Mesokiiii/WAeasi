//! Code section — function bodies (locals + raw bytecode slice).
use alloc::vec::Vec;

use super::reader::Reader;
use super::types::ValType;
use super::ParseError;

#[derive(Debug, Clone)]
pub struct LocalsRun { pub count: u32, pub ty: ValType }

#[derive(Debug, Clone)]
pub struct FunctionBody<'a> {
    pub locals: Vec<LocalsRun>,
    pub code:   &'a [u8],
}

/// Parse the Code section (id=10).
pub fn parse_section<'a>(r: &mut Reader<'a>) -> Result<Vec<FunctionBody<'a>>, ParseError> {
    let n = r.u32_leb()? as usize;
    let mut out = Vec::with_capacity(n.min(super::leb128::MAX_SECTION_COUNT));
    for _ in 0..n {
        let body_size = r.u32_leb()? as usize;
        let body = r.bytes(body_size)?;
        let mut sub = Reader::new(body);
        out.push(parse_body(&mut sub)?);
    }
    Ok(out)
}

fn parse_body<'a>(r: &mut Reader<'a>) -> Result<FunctionBody<'a>, ParseError> {
    let local_runs = r.u32_leb()? as usize;
    let mut locals = Vec::with_capacity(local_runs.min(super::leb128::MAX_SECTION_COUNT));
    for _ in 0..local_runs {
        let count = r.u32_leb()?;
        let ty    = ValType::decode(r.u8()?)?;
        locals.push(LocalsRun { count, ty });
    }
    let code = &r.bytes(r.remaining())?[..];
    Ok(FunctionBody { locals, code })
}

/// Parse the Function section (id=3) — sequence of type indices.
pub fn parse_function_section(r: &mut Reader<'_>) -> Result<Vec<u32>, ParseError> {
    let n = r.u32_leb()? as usize;
    let mut out = Vec::with_capacity(n.min(super::leb128::MAX_SECTION_COUNT));
    for _ in 0..n { out.push(r.u32_leb()?); }
    Ok(out)
}
