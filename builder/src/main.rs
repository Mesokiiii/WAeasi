//! waeasi-builder — language-agnostic CLI driver.
//!
//! Subcommands:
//!   build    drive the full pipeline against a `--component` artefact
//!   inspect  decode a `.waeasi-bundle` header
//!
//! Run `waeasi-builder --help` for full usage.

use std::path::PathBuf;
use std::process::ExitCode;

use waeasi_builder::pipeline::{format_report, parse_key_source, Pipeline, PipelineOptions};
use waeasi_builder::stages::componentize::Language;
use waeasi_builder::{BuildError, Manifest};

const USAGE: &str = "\
waeasi-builder — language-agnostic build pipeline for WAeasi components

Usage: waeasi-builder <command> [options]

Commands:
  build       package a Wasm Component into a signed .waeasi-bundle
  inspect     decode a .waeasi-bundle header
  --version   print version
  --help      this message

Run 'waeasi-builder <command> --help' for command-specific options.
";

const BUILD_USAGE: &str = "\
waeasi-builder build [options]

Required:
  --component <path>     path to the input .wasm (component or core) or
                         language source (.js / .py)
  --manifest  <path>     waeasi.toml-style manifest
  --key       <spec>     env:VAR | file:/path | raw:HEX

Optional:
  --out       <dir>      output directory (default: dist)
  --wit       <dir>      WIT root (required for .js / .py inputs)
  --world     <name>     target world name (default: from manifest, fallback: handler)
  --language  <auto|wasm-component|wasm-core|js|python>  override detection
  --working-dir <dir>    cwd for componentize-py / jco
  --skip-wizer           skip pre-init snapshot
  --split-engine         split engine.cwasm out from user.cwasm
  --aot                  enable AOT (wasmtime compile) — best-effort
  --aot-target <triple>  AOT target triple (e.g. x86_64-unknown-none)
  --sdk       <label>    free-form SDK label written into manifest.sdk
                         (default: builder@<version>)
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print!("{USAGE}");
        return if args.is_empty() { ExitCode::from(2) } else { ExitCode::SUCCESS };
    }
    if args[0] == "--version" || args[0] == "-V" {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return ExitCode::SUCCESS;
    }
    match args[0].as_str() {
        "build"   => match cmd_build(&args[1..])   { Ok(c) => c, Err(e) => report(e) },
        "inspect" => match cmd_inspect(&args[1..]) { Ok(c) => c, Err(e) => report(e) },
        unknown => {
            eprintln!("error unknown command: {unknown}");
            print!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn cmd_build(args: &[String]) -> Result<ExitCode, BuildError> {
    let mut opts = Args::default();
    opts.parse(args)?;

    if opts.help { print!("{BUILD_USAGE}"); return Ok(ExitCode::SUCCESS); }
    let component = opts.require("--component", &opts.component)?;
    let manifest_path = opts.require("--manifest", &opts.manifest)?;
    let key_spec = opts.require("--key", &opts.key)?;

    let manifest = Manifest::from_path(manifest_path)?;
    let key = parse_key_source(key_spec)?;

    let mut po = PipelineOptions::default();
    if let Some(d) = &opts.out_dir   { po.out_dir = d.clone(); }
    po.wit_path     = opts.wit_path.clone();
    po.world        = opts.world.clone()
        .or_else(|| manifest.spec.world.as_ref().map(|w| w.as_str().into()))
        .unwrap_or_else(|| "handler".into());
    po.language     = opts.language;
    po.working_dir  = opts.working_dir.clone();
    po.skip_wizer   = opts.skip_wizer;
    po.split_engine = opts.split_engine;
    po.aot          = opts.aot;
    po.aot_target   = opts.aot_target.clone();
    if let Some(s) = &opts.sdk_label { po.sdk_label = s.clone(); }

    let pipeline = Pipeline::new(po)?;
    let report = pipeline.run(component, &manifest, &key)?;
    print!("{}", format_report(&report));
    Ok(ExitCode::SUCCESS)
}

fn cmd_inspect(args: &[String]) -> Result<ExitCode, BuildError> {
    if args.is_empty() {
        eprintln!("usage: waeasi-builder inspect <bundle.waeasi-bundle>");
        return Ok(ExitCode::from(2));
    }
    let path = std::path::Path::new(&args[0]);
    let bytes = waeasi_builder::fs::read(path)?;
    let entries = decode_header(&bytes)?;
    println!("bundle  {}", path.display());
    println!("size    {} bytes", bytes.len());
    println!("entries {}", entries.len());
    for (name, off, len) in &entries {
        println!("  {name:<22} offset={off:<9} length={len:<9}");
    }
    Ok(ExitCode::SUCCESS)
}

fn decode_header(b: &[u8]) -> Result<Vec<(String, u64, u64)>, BuildError> {
    if b.len() < 14 || &b[0..9] != b"WAEASIBND" {
        return Err(BuildError::Bundle("bad magic".into()));
    }
    let count = u32::from_le_bytes([b[10], b[11], b[12], b[13]]) as usize;
    let mut off = 14usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if off >= b.len() { return Err(BuildError::Bundle("truncated header".into())); }
        let nlen = b[off] as usize; off += 1;
        if off + nlen + 16 > b.len() {
            return Err(BuildError::Bundle("truncated entry".into()));
        }
        let name = std::str::from_utf8(&b[off..off + nlen])
            .map_err(|_| BuildError::Bundle("bad utf8 in name".into()))?
            .to_string();
        off += nlen;
        let e_off = u64::from_le_bytes(b[off..off + 8].try_into().unwrap()); off += 8;
        let e_len = u64::from_le_bytes(b[off..off + 8].try_into().unwrap()); off += 8;
        out.push((name, e_off, e_len));
    }
    Ok(out)
}

#[derive(Default)]
struct Args {
    component:    Option<PathBuf>,
    manifest:     Option<PathBuf>,
    key:          Option<String>,
    out_dir:      Option<PathBuf>,
    wit_path:     Option<PathBuf>,
    world:        Option<String>,
    language:     Language,
    working_dir:  Option<PathBuf>,
    skip_wizer:   bool,
    split_engine: bool,
    aot:          bool,
    aot_target:   Option<String>,
    sdk_label:    Option<String>,
    help:         bool,
}

impl Args {
    fn parse(&mut self, args: &[String]) -> Result<(), BuildError> {
        let mut i = 0;
        while i < args.len() {
            let a = args[i].as_str();
            let next = || -> Result<&String, BuildError> {
                args.get(i + 1).ok_or_else(||
                    BuildError::Manifest(format!("--{a} expects a value")))
            };
            match a {
                "--component"   => { self.component   = Some(next()?.into()); i += 2; }
                "--manifest"    => { self.manifest    = Some(next()?.into()); i += 2; }
                "--key"         => { self.key         = Some(next()?.into()); i += 2; }
                "--out"         => { self.out_dir     = Some(next()?.into()); i += 2; }
                "--wit"         => { self.wit_path    = Some(next()?.into()); i += 2; }
                "--world"       => { self.world       = Some(next()?.clone()); i += 2; }
                "--language"    => { self.language = parse_lang(next()?)?; i += 2; }
                "--working-dir" => { self.working_dir = Some(next()?.into()); i += 2; }
                "--aot-target"  => { self.aot_target  = Some(next()?.clone()); i += 2; }
                "--sdk"         => { self.sdk_label   = Some(next()?.clone()); i += 2; }
                "--skip-wizer"   => { self.skip_wizer   = true; i += 1; }
                "--split-engine" => { self.split_engine = true; i += 1; }
                "--aot"          => { self.aot          = true; i += 1; }
                "--help" | "-h"  => { self.help         = true; i += 1; }
                other => return Err(BuildError::Manifest(format!("unknown flag: {other}"))),
            }
        }
        Ok(())
    }

    fn require<'a, T>(&self, flag: &str, opt: &'a Option<T>) -> Result<&'a T, BuildError> {
        opt.as_ref().ok_or_else(|| BuildError::Manifest(format!("{flag} required")))
    }
}

fn parse_lang(s: &str) -> Result<Language, BuildError> {
    Ok(match s {
        "auto"           => Language::Auto,
        "wasm-component" => Language::WasmComponent,
        "wasm-core"      => Language::WasmCore,
        "js"             => Language::Js,
        "python"         => Language::Python,
        o => return Err(BuildError::Manifest(format!("unknown language: {o}"))),
    })
}

fn report(e: BuildError) -> ExitCode {
    eprintln!("error {e}");
    ExitCode::from(3)
}
