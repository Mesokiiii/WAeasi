//! `waeasictl wasm validate|disasm <path>` — local Wasm tooling.
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    let sub = args.first().map(|s| s.as_str())
        .ok_or_else(|| CliError::Usage("wasm <validate|disasm> <path>".into()))?;
    match sub {
        "validate" => validate(&args[1..]),
        "disasm"   => disasm(&args[1..]),
        u          => Err(CliError::Usage(format!("unknown wasm sub: {}", u))),
    }
}

fn validate(args: &[String]) -> CliResult {
    let path = args.first().ok_or_else(|| CliError::Usage("wasm validate <path>".into()))?;
    let bytes = std::fs::read(path).map_err(|e| CliError::Io(format!("read {}: {}", path, e)))?;
    if bytes.len() < 8 || &bytes[..4] != b"\0asm" {
        return Err(CliError::Runtime(format!("not a Wasm file: {}", path)));
    }
    let v = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if v != 1 {
        return Err(CliError::Runtime(format!("unsupported Wasm version: {}", v)));
    }
    println!("ok: {} bytes, Wasm v1", bytes.len());
    Ok(())
}

fn disasm(args: &[String]) -> CliResult {
    let path = args.first().ok_or_else(|| CliError::Usage("wasm disasm <path>".into()))?;
    let bytes = std::fs::read(path).map_err(|e| CliError::Io(format!("read {}: {}", path, e)))?;
    if bytes.len() < 8 || &bytes[..4] != b"\0asm" {
        return Err(CliError::Runtime(format!("not a Wasm file: {}", path)));
    }
    println!("magic   : \\0asm");
    println!("version : {}", u32::from_le_bytes(bytes[4..8].try_into().unwrap()));
    let mut p = 8usize;
    while p < bytes.len() {
        let id = bytes[p]; p += 1;
        let (size, used) = read_leb_u32(&bytes[p..]);
        p += used;
        println!("section : id={:>2} ({:<10}) size={}", id, section_name(id), size);
        p += size as usize;
    }
    Ok(())
}

fn read_leb_u32(buf: &[u8]) -> (u32, usize) {
    let mut value = 0u32; let mut shift = 0; let mut used = 0;
    for &b in buf.iter().take(5) {
        used += 1;
        value |= ((b & 0x7F) as u32) << shift;
        if b & 0x80 == 0 { break; }
        shift += 7;
    }
    (value, used)
}

fn section_name(id: u8) -> &'static str {
    match id {
        0  => "Custom",   1  => "Type",     2  => "Import",
        3  => "Function", 4  => "Table",    5  => "Memory",
        6  => "Global",   7  => "Export",   8  => "Start",
        9  => "Element",  10 => "Code",     11 => "Data",
        12 => "DataCount", _ => "?",
    }
}
