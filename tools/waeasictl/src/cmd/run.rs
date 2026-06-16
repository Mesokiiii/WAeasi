//! `waeasictl run <manifest.toml>` — load + start a component.
use crate::conn::Conn;
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    let manifest_path = args.first()
        .ok_or_else(|| CliError::Usage("run <manifest.toml>".into()))?;

    let manifest = std::fs::read_to_string(manifest_path)
        .map_err(|e| CliError::Io(format!("read {}: {}", manifest_path, e)))?;
    let wasm_path = manifest_path.trim_end_matches(".toml").to_string() + ".wasm";
    let wasm = std::fs::read(&wasm_path)
        .map_err(|e| CliError::Io(format!("read {}: {}", wasm_path, e)))?;

    let header = format!("RUN\n{}\n{}\n{}\n", manifest.len(), manifest, wasm.len());
    let mut c = Conn::open_default()?;
    c.write_all(header.as_bytes())?;
    c.write_all(&wasm)?;
    let reply = c.read_to_string()?;
    println!("{}", reply.trim());
    Ok(())
}
