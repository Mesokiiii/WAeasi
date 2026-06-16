//! Table section parser (id=4).
//!
//! Wasm tables are typed reference arrays — required for `call_indirect`.
//! Stage 3 only needs `funcref` tables for indirect calls; `externref`
//! is parsed but not yet executed.
use alloc::vec::Vec;

use super::imports::parse_limits;
use super::reader::Reader;
use super::types::ValType;
use super::ParseError;

#[derive(Debug, Clone)]
pub struct TableType {
    pub elem: ValType,
    pub min:  u32,
    pub max:  Option<u32>,
}

pub fn parse_section(r: &mut Reader<'_>) -> Result<Vec<TableType>, ParseError> {
    let n = r.u32_leb()? as usize;
    let mut out = Vec::with_capacity(n.min(super::leb128::MAX_SECTION_COUNT));
    for _ in 0..n {
        let elem = ValType::decode(r.u8()?)?;
        if elem != ValType::FuncRef && elem != ValType::ExternRef {
            return Err(ParseError::UnsupportedFeature("non-ref table"));
        }
        let (min, max) = parse_limits(r)?;
        out.push(TableType { elem, min, max });
    }
    Ok(out)
}
