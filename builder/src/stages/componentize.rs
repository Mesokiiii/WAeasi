//! componentize stage — drives language-specific tooling.
//!
//! For inputs that are already a Wasm Component (TinyGo, Rust+wit-bindgen)
//! this stage is a no-op pass-through.  For raw JS/TS files it shells
//! out to `jco componentize`; for Python entry points to `componentize-py`.
//!
//! Detection is by file extension; the user can override via
//! `Input::language`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::digest;
use crate::error::{BuildError, Result};
use crate::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language { WasmComponent, WasmCore, Js, Python, #[default] Auto }

#[derive(Debug, Clone)]
pub struct Input<'a> {
    pub source:    &'a Path,
    pub out_path:  &'a Path,
    pub wit_path:  Option<&'a Path>,
    pub world:     &'a str,
    pub language:  Language,
    pub working_dir: Option<&'a Path>,
}

#[derive(Debug)]
pub struct Output {
    pub component_path: PathBuf,
    pub size_bytes:     u64,
    pub duration_ms:    u128,
}

pub fn run(input: &Input<'_>) -> Result<Output> {
    let start = Instant::now();
    let lang = match input.language {
        Language::Auto => detect(input.source)?,
        l => l,
    };
    match lang {
        Language::WasmComponent | Language::WasmCore => {
            // Pass-through: copy the artefact under the canonical name.
            fs::copy(input.source, input.out_path)?;
        }
        Language::Js     => run_jco(input)?,
        Language::Python => run_componentize_py(input)?,
        Language::Auto   => unreachable!(),
    }

    if !digest::is_component(input.out_path)? {
        return Err(BuildError::BadComponent(format!(
            "{} is not a Wasm Component (only Component Model accepted)",
            input.out_path.display(),
        )));
    }

    Ok(Output {
        component_path: input.out_path.into(),
        size_bytes:     fs::size(input.out_path)?,
        duration_ms:    start.elapsed().as_millis(),
    })
}

fn detect(p: &Path) -> Result<Language> {
    if let Some(ext) = p.extension().and_then(|x| x.to_str()) {
        return Ok(match ext {
            "wasm"            => detect_wasm(p)?,
            "js" | "mjs"      => Language::Js,
            "py"              => Language::Python,
            o => return Err(BuildError::Toolchain(format!("can't detect language from .{o}"))),
        });
    }
    Err(BuildError::Toolchain("source has no extension".into()))
}

fn detect_wasm(p: &Path) -> Result<Language> {
    Ok(if digest::is_component(p)? { Language::WasmComponent } else { Language::WasmCore })
}

fn run_jco(input: &Input<'_>) -> Result<()> {
    let wit = input.wit_path.ok_or_else(||
        BuildError::Toolchain("--wit is required for JS sources".into())
    )?;
    let bin = std::env::var_os("WAEASI_JCO").unwrap_or_else(|| "jco".into());
    let mut cmd = Command::new(&bin);
    cmd.arg("componentize")
        .arg(input.source)
        .arg("--wit").arg(wit)
        .arg("--world-name").arg(input.world)
        .arg("-o").arg(input.out_path);
    if let Some(d) = input.working_dir { cmd.current_dir(d); }
    finalize(cmd, "jco")
}

fn run_componentize_py(input: &Input<'_>) -> Result<()> {
    let wit = input.wit_path.ok_or_else(||
        BuildError::Toolchain("--wit is required for Python sources".into())
    )?;
    let bin = std::env::var_os("WAEASI_COMPONENTIZE_PY")
        .unwrap_or_else(|| "componentize-py".into());

    let module = input.source.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| BuildError::Toolchain("bad python module name".into()))?;

    let mut cmd = Command::new(&bin);
    cmd.arg("-d").arg(wit)
        .arg("-w").arg(input.world)
        .arg("componentize")
        .arg(module)
        .arg("-o").arg(input.out_path);
    if let Some(d) = input.working_dir { cmd.current_dir(d); }
    finalize(cmd, "componentize-py")
}

fn finalize(mut cmd: Command, tool: &str) -> Result<()> {
    let out = cmd.output().map_err(|e| BuildError::Toolchain(
        format!("could not execute {tool}: {e}"),
    ))?;
    if !out.status.success() {
        return Err(BuildError::Toolchain(format!(
            "{} failed (exit {:?}):\n{}",
            tool, out.status.code(),
            String::from_utf8_lossy(&out.stderr),
        )));
    }
    Ok(())
}
