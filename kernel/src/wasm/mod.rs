//! Wasm runtime — Stage 7 wires hot-reload alongside the existing
//! parser + validator + interp + JIT pipeline.
pub mod engine;
pub mod hot_reload;
pub mod instance;
pub mod interp;
pub mod linker;
pub mod module;
pub mod parser;
pub mod store;
pub mod trap;
pub mod validator;

pub fn init() {
    engine::init();
    linker::init();
    log::info!("[wasm] engine ready (parser + validator + interp + hot-reload)");
}

#[derive(Debug)]
pub enum WasmError {
    Decode,
    Validate,
    Link(&'static str),
    OutOfMemory,
    Trap(trap::Trap),
    Verify(crate::crypto::verify::VerifyError),
    Parse(parser::ParseError),
}
