//! aot stage — offline native code generation.
//!
//! When the kernel boots a component it would normally JIT-compile via
//! Cranelift.  For latency-sensitive deployments we let the builder
//! pre-compile against a fixed Cranelift version in CI, content-address
//! the output, and ship `.cwasm.aot` next to the component.  The
//! kernel then mmaps the precompiled native code at load time and
//! skips JIT entirely.
//!
//! This stage is a *placeholder*: it shells out to `wasmtime compile`
//! when available (the canonical AOT tool with the Cranelift backend),
//! caches the result by user-digest, and reports back.  In Stage 11 of
//! the kernel roadmap, the bytes emitted here are accepted directly by
//! `kernel/src/wasm/code_cache`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::error::{BuildError, Result};
use crate::fs;

#[derive(Debug, Clone)]
pub struct Input<'a> {
    pub component: &'a Path,
    pub out_dir:   &'a Path,
    pub user_digest: &'a str,
    pub enabled:   bool,
    pub optional:  bool,    // skip when wasmtime not on PATH
    pub target_triple: Option<&'a str>, // e.g. "x86_64-unknown-none"
}

#[derive(Debug)]
pub struct Output {
    pub aot_path:    Option<PathBuf>,
    pub size_bytes:  u64,
    pub duration_ms: u128,
    pub skipped:     bool,
    pub reason:      Option<&'static str>,
}

pub fn run(input: &Input<'_>) -> Result<Output> {
    let start = Instant::now();
    if !input.enabled {
        return Ok(skipped(start, "disabled"));
    }

    let bin = std::env::var_os("WAEASI_WASMTIME")
        .unwrap_or_else(|| "wasmtime".into());

    let out_path = input.out_dir.join(format!("{}.cwasm.aot", &input.user_digest[..16]));
    let mut cmd = Command::new(&bin);
    cmd.arg("compile");
    if let Some(triple) = input.target_triple {
        cmd.arg("--target").arg(triple);
    }
    cmd.arg(input.component).arg("-o").arg(&out_path);

    let res = cmd.output();
    let out = match res {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && input.optional => {
            return Ok(skipped(start, "wasmtime not installed"));
        }
        Err(e) => return Err(BuildError::Toolchain(
            format!("wasmtime compile: {e}"),
        )),
    };
    if !out.status.success() {
        if input.optional {
            return Ok(skipped(start, "wasmtime compile failed"));
        }
        return Err(BuildError::Toolchain(format!(
            "wasmtime compile failed (exit {:?}):\n{}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        )));
    }

    let size = fs::size(&out_path)?;
    Ok(Output {
        aot_path:    Some(out_path),
        size_bytes:  size,
        duration_ms: start.elapsed().as_millis(),
        skipped:     false,
        reason:      None,
    })
}

fn skipped(start: Instant, reason: &'static str) -> Output {
    Output {
        aot_path:    None,
        size_bytes:  0,
        duration_ms: start.elapsed().as_millis(),
        skipped:     true,
        reason:      Some(reason),
    }
}
