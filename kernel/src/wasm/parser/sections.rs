//! Top-level section dispatcher.
//!
//! Walks the binary front-to-back, dispatching each section to its
//! specialized parser.  Section ids must appear in canonical order
//! (Wasm spec § 5.5) — duplicate or out-of-order is rejected.
use super::code;
use super::element;
use super::imports::{self, parse_limits};
use super::reader::Reader;
use super::table;
use super::types;
use super::{ParseError, ParsedModule};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SectionKind {
    Custom = 0, Type = 1, Import = 2, Function = 3, Table = 4,
    Memory = 5, Global = 6, Export = 7, Start = 8, Element = 9,
    Code = 10, Data = 11, DataCount = 12,
}

impl SectionKind {
    pub fn from_id(id: u8) -> Option<SectionKind> {
        match id {
            0  => Some(Self::Custom),  1  => Some(Self::Type),
            2  => Some(Self::Import),  3  => Some(Self::Function),
            4  => Some(Self::Table),   5  => Some(Self::Memory),
            6  => Some(Self::Global),  7  => Some(Self::Export),
            8  => Some(Self::Start),   9  => Some(Self::Element),
            10 => Some(Self::Code),    11 => Some(Self::Data),
            12 => Some(Self::DataCount),
            _  => None,
        }
    }
}

pub fn parse_module(bytes: &[u8]) -> Result<ParsedModule<'_>, ParseError> {
    if bytes.len() < 8 { return Err(ParseError::UnexpectedEof); }
    if &bytes[..4] != b"\0asm" { return Err(ParseError::BadMagic); }
    if u32::from_le_bytes(bytes[4..8].try_into().unwrap()) != 1 {
        return Err(ParseError::BadVersion);
    }

    let mut module = ParsedModule {
        types:     alloc::vec::Vec::with_capacity(16),
        imports:   alloc::vec::Vec::with_capacity(16),
        functions: alloc::vec::Vec::with_capacity(64),
        bodies:    alloc::vec::Vec::with_capacity(64),
        tables:    alloc::vec::Vec::with_capacity(2),
        elements:  alloc::vec::Vec::with_capacity(4),
        memory_initial: 0, memory_max: None, start_func: None,
    };

    let mut r = Reader::new(&bytes[8..]);
    let mut last_id: i32 = -1;

    while !r.eof() {
        let id = r.u8()?;
        let size = r.u32_leb()? as usize;
        let kind = SectionKind::from_id(id).ok_or(ParseError::BadSection(id))?;
        if kind != SectionKind::Custom {
            if (id as i32) <= last_id { return Err(ParseError::DuplicateSection(kind)); }
            last_id = id as i32;
        }
        r.limited(size, |sub| dispatch(kind, sub, &mut module))?;
    }
    Ok(module)
}

fn dispatch<'a>(
    kind: SectionKind,
    r: &mut Reader<'a>,
    m: &mut ParsedModule<'a>,
) -> Result<(), ParseError> {
    match kind {
        SectionKind::Custom   => { /* skip */ }
        SectionKind::Type     => m.types     = types::parse_section(r)?,
        SectionKind::Import   => m.imports   = imports::parse_section(r)?,
        SectionKind::Function => m.functions = code::parse_function_section(r)?,
        SectionKind::Table    => m.tables    = table::parse_section(r)?,
        SectionKind::Memory   => parse_memory(r, m)?,
        SectionKind::Element  => m.elements  = element::parse_section(r)?,
        SectionKind::Start    => m.start_func = Some(r.u32_leb()?),
        SectionKind::Code     => m.bodies    = code::parse_section(r)?,
        _ => { let _ = r.bytes(r.remaining())?; }
    }
    Ok(())
}

fn parse_memory(r: &mut Reader<'_>, m: &mut ParsedModule<'_>) -> Result<(), ParseError> {
    let n = r.u32_leb()?;
    if n > 1 { return Err(ParseError::UnsupportedFeature("multi-memory")); }
    if n == 1 {
        let (min, max) = parse_limits(r)?;
        m.memory_initial = min;
        m.memory_max     = max;
    }
    Ok(())
}
