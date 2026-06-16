//! wizer stage — pre-init snapshot via the external `wizer` binary.
//!
//! Optional.  When `skip == true` (or wizer isn't installed and
//! `optional` is set) we copy the input verbatim.  This matters most
//! for JS/Python; Go and Rust components rarely benefit.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::error::{BuildError, Result};
use crate::fs;

#[derive(Debug, Clone)]
pub struct Input<'a> {
    pub component:  &'a Path,
    pub out_path:   &'a Path,
    pub init_func:  &'a str,        // "wizer.initialize" by default
    pub allow_wasi: bool,
    pub timeout_s:  u64,
    pub skip:       bool,
    pub optional:   bool,           // pass-through when wizer is missing
}

impl<'a> Input<'a> {
    pub fn new(component: &'a Path, out_path: &'a Path) -> Self {
        Self {
            component, out_path,
            init_func: "wizer.initialize",
            allow_wasi: true,
            timeout_s:  60,
            skip: false,
            optional: false,
        }
    }
}

#[derive(Debug)]
pub struct Output {
    pub snapshot_path: PathBuf,
    pub size_bytes:    u64,
    pub duration_ms:   u128,
    pub grew:          bool,
}

pub fn run(input: &Input<'_>) -> Result<Output> {
    let start = Instant::now();
    if input.skip {
        return passthrough(input, start);
    }
    let bin = std::env::var_os("WAEASI_WIZER").unwrap_or_else(|| "wizer".into());

    let in_size = fs::size(input.component)?;
    let mut cmd = Command::new(&bin);
    cmd.arg(input.component)
       .arg("-o").arg(input.out_path)
       .arg("--init-func").arg(input.init_func)
       .arg("--allow-wasi").arg(if input.allow_wasi { "true" } else { "false" })
       .arg("--wasm-bulk-memory").arg("true");

    let out = cmd.output();
    let out = match out {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && input.optional => {
            return passthrough(input, start);
        }
        Err(e) => return Err(BuildError::Wizer(
            format!("could not execute wizer: {e}"),
        )),
    };
    if !out.status.success() {
        return Err(BuildError::Wizer(format!(
            "wizer failed (exit {:?}):\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        )));
    }

    let out_size = fs::size(input.out_path)?;
    Ok(Output {
        snapshot_path: input.out_path.into(),
        size_bytes:    out_size,
        duration_ms:   start.elapsed().as_millis(),
        grew:          out_size > in_size,
    })
}

fn passthrough(input: &Input<'_>, start: Instant) -> Result<Output> {
    fs::copy(input.component, input.out_path)?;
    Ok(Output {
        snapshot_path: input.out_path.into(),
        size_bytes:    fs::size(input.out_path)?,
        duration_ms:   start.elapsed().as_millis(),
        grew:          false,
    })
}
