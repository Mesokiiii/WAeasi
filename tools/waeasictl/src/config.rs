//! Persistent CLI config at `~/.waeasi/config.toml`.
//!
//! ```toml
//!   server   = "10.0.0.5:9300"
//!   output   = "json"
//!   color    = "always"
//!   verbose  = false
//! ```
//!
//! Resolution order (most specific wins):
//!   1. CLI flag (`--server`, `--output`, ...)
//!   2. Environment variable (`WAEASI_ADMIN`, `WAEASI_OUTPUT`)
//!   3. `~/.waeasi/config.toml`
//!   4. Built-in defaults
use std::fs;
use std::path::PathBuf;

use crate::color::Mode as ColorMode;
use crate::error::{CliError, CliResult};

#[derive(Debug, Clone)]
pub struct Config {
    pub server:  String,
    pub output:  Output,
    pub color:   ColorMode,
    pub verbose: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Output { Table, Wide, Json, Yaml }

impl Default for Config {
    fn default() -> Self {
        Self {
            server:  String::from("127.0.0.1:9300"),
            output:  Output::Table,
            color:   ColorMode::Auto,
            verbose: false,
        }
    }
}

pub fn path() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .unwrap_or_default();
    let mut p = PathBuf::from(home);
    p.push(".waeasi");
    p.push("config.toml");
    p
}

/// Load from disk + apply env-var overrides.  Missing file is OK
/// (returns defaults).  Malformed file is `CliError::Config`.
pub fn load() -> CliResult<Config> {
    let mut cfg = Config::default();
    let p = path();
    if p.exists() {
        let raw = fs::read_to_string(&p)
            .map_err(|e| CliError::Config(format!("read {}: {}", p.display(), e)))?;
        apply_toml(&mut cfg, &raw)?;
    }
    apply_env(&mut cfg);
    Ok(cfg)
}

/// Save to disk — creates directory + file if needed.
pub fn save(cfg: &Config) -> CliResult {
    let p = path();
    if let Some(dir) = p.parent() { let _ = fs::create_dir_all(dir); }
    let mut s = String::with_capacity(128);
    s.push_str(&format!("server  = \"{}\"\n", cfg.server));
    s.push_str(&format!("output  = \"{}\"\n", output_str(cfg.output)));
    s.push_str(&format!("color   = \"{}\"\n", color_str(cfg.color)));
    s.push_str(&format!("verbose = {}\n",     cfg.verbose));
    fs::write(&p, s).map_err(|e| CliError::Config(format!("write {}: {}", p.display(), e)))
}

fn apply_toml(cfg: &mut Config, raw: &str) -> CliResult {
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let (k, v) = line.split_once('=')
            .ok_or_else(|| CliError::Config(format!("bad line: {}", line)))?;
        let key = k.trim();
        let val = v.trim();
        match key {
            "server"  => cfg.server  = strip_quotes(val).to_string(),
            "output"  => cfg.output  = parse_output(strip_quotes(val))?,
            "color"   => cfg.color   = parse_color(strip_quotes(val))?,
            "verbose" => cfg.verbose = val == "true",
            _ => {}
        }
    }
    Ok(())
}

fn apply_env(cfg: &mut Config) {
    if let Ok(s) = std::env::var("WAEASI_ADMIN")   { cfg.server  = s; }
    if let Ok(o) = std::env::var("WAEASI_OUTPUT")  { if let Ok(p) = parse_output(&o)  { cfg.output = p; } }
    if let Ok(c) = std::env::var("WAEASI_COLOR")   { if let Ok(p) = parse_color(&c)   { cfg.color  = p; } }
    if let Ok(v) = std::env::var("WAEASI_VERBOSE") { cfg.verbose = v == "1" || v == "true"; }
}

fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') { &s[1..s.len()-1] } else { s }
}

fn parse_output(s: &str) -> CliResult<Output> {
    match s {
        "table" => Ok(Output::Table),
        "wide"  => Ok(Output::Wide),
        "json"  => Ok(Output::Json),
        "yaml"  => Ok(Output::Yaml),
        _ => Err(CliError::Config(format!("bad output: {}", s))),
    }
}

fn parse_color(s: &str) -> CliResult<ColorMode> {
    match s {
        "auto"   => Ok(ColorMode::Auto),
        "always" => Ok(ColorMode::Always),
        "never"  => Ok(ColorMode::Never),
        _ => Err(CliError::Config(format!("bad color: {}", s))),
    }
}

fn output_str(o: Output) -> &'static str {
    match o { Output::Table => "table", Output::Wide => "wide", Output::Json => "json", Output::Yaml => "yaml" }
}
fn color_str(c: ColorMode) -> &'static str {
    match c { ColorMode::Auto => "auto", ColorMode::Always => "always", ColorMode::Never => "never" }
}
