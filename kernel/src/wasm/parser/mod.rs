//! Wasm-binary parser — single-pass, zero-copy where possible.
pub mod code;
pub mod element;
pub mod imports;
pub mod leb128;
pub mod reader;
pub mod sections;
pub mod table;
pub mod types;

use alloc::vec::Vec;

use code::FunctionBody;
use element::ElementSegment;
use imports::Import;
use sections::SectionKind;
use table::TableType;
use types::FuncType;

#[derive(Debug)]
pub struct ParsedModule<'a> {
    pub types:     Vec<FuncType>,
    pub imports:   Vec<Import<'a>>,
    pub functions: Vec<u32>,
    pub bodies:    Vec<FunctionBody<'a>>,
    pub tables:    Vec<TableType>,
    pub elements:  Vec<ElementSegment>,
    pub memory_initial: u32,
    pub memory_max:     Option<u32>,
    pub start_func:     Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    BadMagic, BadVersion, UnexpectedEof, BadLeb128,
    BadSection(u8), BadValType(u8), BadImportKind(u8),
    DuplicateSection(SectionKind), Truncated,
    UnsupportedFeature(&'static str),
}

pub fn parse(bytes: &[u8]) -> Result<ParsedModule<'_>, ParseError> {
    sections::parse_module(bytes)
}

pub use leb128::{read_u32 as leb_u32, read_i32 as leb_i32, read_i64 as leb_i64};
pub use reader::Reader;
