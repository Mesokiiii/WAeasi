//! Element section parser (id=9).
//!
//! Elements populate Wasm tables with function references at module
//! instantiation.  Stage 3 supports the **active**, MVP-style segment:
//! `segment.flags == 0` (table 0, i32.const offset, vec<funcidx>).
//!
//! Other segment flavours (passive, declarative, table-index, expressions)
//! are returned as `Unsupported`.
use alloc::vec::Vec;

use super::reader::Reader;
use super::ParseError;

#[derive(Debug, Clone)]
pub struct ElementSegment {
    pub table_idx:  u32,
    pub offset_pc:  i32,           // i32.const value of the offset expr
    pub func_idxs:  Vec<u32>,
}

pub fn parse_section(r: &mut Reader<'_>) -> Result<Vec<ElementSegment>, ParseError> {
    let n = r.u32_leb()? as usize;
    let mut out = Vec::with_capacity(n.min(super::leb128::MAX_SECTION_COUNT));
    for _ in 0..n {
        let flags = r.u32_leb()?;
        if flags != 0 {
            return Err(ParseError::UnsupportedFeature("non-MVP element segment"));
        }
        let opc = r.u8()?;
        if opc != 0x41 { return Err(ParseError::UnsupportedFeature("non-i32.const offset")); }
        let off = r.i32_leb()?;
        let end = r.u8()?;
        if end != 0x0B { return Err(ParseError::UnsupportedFeature("missing end on offset expr")); }

        let m = r.u32_leb()? as usize;
        let mut funcs = Vec::with_capacity(m.min(super::leb128::MAX_SECTION_COUNT));
        for _ in 0..m { funcs.push(r.u32_leb()?); }
        out.push(ElementSegment { table_idx: 0, offset_pc: off, func_idxs: funcs });
    }
    Ok(out)
}
