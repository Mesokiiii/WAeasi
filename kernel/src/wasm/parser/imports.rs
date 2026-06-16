//! Imports section parser.
use alloc::vec::Vec;

use super::reader::Reader;
use super::types::ValType;
use super::ParseError;

#[derive(Debug, Clone)]
pub struct Import<'a> {
    pub module: &'a str,
    pub field:  &'a str,
    pub kind:   ImportKind,
}

#[derive(Debug, Clone)]
pub enum ImportKind {
    Function { type_idx: u32 },
    Table    { elem: ValType, min: u32, max: Option<u32> },
    Memory   { min: u32, max: Option<u32> },
    Global   { ty: ValType, mutable: bool },
}

/// Parse the Import section (id=2).
pub fn parse_section<'a>(r: &mut Reader<'a>) -> Result<Vec<Import<'a>>, ParseError> {
    let n = r.u32_leb()? as usize;
    let mut out = Vec::with_capacity(n.min(super::leb128::MAX_SECTION_COUNT));
    for _ in 0..n {
        let module = r.name()?;
        let field  = r.name()?;
        let kind_byte = r.u8()?;
        let kind = match kind_byte {
            0x00 => ImportKind::Function { type_idx: r.u32_leb()? },
            0x01 => {
                let elem = ValType::decode(r.u8()?)?;
                let (min, max) = parse_limits(r)?;
                ImportKind::Table { elem, min, max }
            }
            0x02 => {
                let (min, max) = parse_limits(r)?;
                ImportKind::Memory { min, max }
            }
            0x03 => {
                let ty = ValType::decode(r.u8()?)?;
                let mutable = match r.u8()? {
                    0 => false, 1 => true, b => return Err(ParseError::BadValType(b)),
                };
                ImportKind::Global { ty, mutable }
            }
            b => return Err(ParseError::BadImportKind(b)),
        };
        out.push(Import { module, field, kind });
    }
    Ok(out)
}

pub fn parse_limits(r: &mut Reader<'_>) -> Result<(u32, Option<u32>), ParseError> {
    let flag = r.u8()?;
    let min = r.u32_leb()?;
    let max = if flag & 1 != 0 { Some(r.u32_leb()?) } else { None };
    Ok((min, max))
}
