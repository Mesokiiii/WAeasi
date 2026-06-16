//! Compiled Wasm module — Stage 2 stores the parsed AST + verified hash.
use alloc::string::String;
use alloc::vec::Vec;

use crate::crypto::verify;
use super::WasmError;

#[derive(Debug)]
pub struct Module {
    pub name:        String,
    pub bytes:       Vec<u8>,
    pub digest:      [u8; 32],
    pub initial_mem: u32,
    pub max_mem:     Option<u32>,
}

impl Module {
    /// Decode + cryptographically verify + structurally validate.
    pub fn compile(name: &str, bytes: &[u8]) -> Result<Self, WasmError> {
        let digest = verify::verify_module(name, bytes)
            .map_err(WasmError::Verify)?;
        let parsed = super::parser::parse(bytes).map_err(WasmError::Parse)?;
        super::validator::validate(&parsed)
            .map_err(|_| WasmError::Validate)?;

        Ok(Self {
            name: String::from(name),
            bytes: bytes.into(),
            digest,
            initial_mem: parsed.memory_initial,
            max_mem:     parsed.memory_max,
        })
    }
}
