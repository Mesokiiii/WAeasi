//! Wasm value-types and function-types.
use alloc::vec::Vec;

use super::reader::Reader;
use super::ParseError;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ValType { I32, I64, F32, F64, FuncRef, ExternRef }

impl ValType {
    pub fn decode(b: u8) -> Result<Self, ParseError> {
        match b {
            0x7F => Ok(ValType::I32),
            0x7E => Ok(ValType::I64),
            0x7D => Ok(ValType::F32),
            0x7C => Ok(ValType::F64),
            0x70 => Ok(ValType::FuncRef),
            0x6F => Ok(ValType::ExternRef),
            _    => Err(ParseError::BadValType(b)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FuncType {
    pub params:  Vec<ValType>,
    pub results: Vec<ValType>,
}

/// Parse a Type section (id=1).  Returns `Vec<FuncType>`.
pub fn parse_section(r: &mut Reader<'_>) -> Result<Vec<FuncType>, ParseError> {
    let n = r.u32_leb()? as usize;
    let mut out = Vec::with_capacity(n.min(super::leb128::MAX_SECTION_COUNT));
    for _ in 0..n {
        let leading = r.u8()?;
        if leading != 0x60 { return Err(ParseError::UnsupportedFeature("non-func type")); }
        let pcount = r.u32_leb()? as usize;
        let mut params = Vec::with_capacity(pcount.min(super::leb128::MAX_SECTION_COUNT));
        for _ in 0..pcount { params.push(ValType::decode(r.u8()?)?); }
        let rcount = r.u32_leb()? as usize;
        let mut results = Vec::with_capacity(rcount.min(super::leb128::MAX_SECTION_COUNT));
        for _ in 0..rcount { results.push(ValType::decode(r.u8()?)?); }
        out.push(FuncType { params, results });
    }
    Ok(out)
}

/// Block-type used by `block / loop / if`: an optional value-type or a
/// reference to a `FuncType` index.
#[derive(Copy, Clone, Debug)]
pub enum BlockType { Empty, Val(ValType), TypeIdx(u32) }

impl BlockType {
    /// Decode a block-type byte stream from the reader's current
    /// position.  The encoding is overloaded:
    ///   * `0x40`             → Empty
    ///   * `0x7F..=0x6F`      → Val(ValType)
    ///   * negative-LEB int   → TypeIdx (reinterpreted from signed)
    ///
    /// Stage 3 supports only Empty / Val; multi-value `TypeIdx` requires
    /// the Wasm 2.0 multi-value extension and lands in stage 4.
    pub fn decode(r: &mut Reader<'_>) -> Result<Self, ParseError> {
        let b = r.u8()?;
        match b {
            0x40 => Ok(BlockType::Empty),
            0x7F | 0x7E | 0x7D | 0x7C | 0x70 | 0x6F => Ok(BlockType::Val(ValType::decode(b)?)),
            _ => Err(ParseError::UnsupportedFeature("multi-value block-type")),
        }
    }
}
