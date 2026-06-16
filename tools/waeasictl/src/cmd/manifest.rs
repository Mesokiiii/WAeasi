//! `waeasictl manifest validate|fmt <path>`.
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    let sub = args.first().map(|s| s.as_str())
        .ok_or_else(|| CliError::Usage("manifest <validate|fmt> <path>".into()))?;
    match sub {
        "validate" => validate(&args[1..]),
        "fmt"      => fmt(&args[1..]),
        u          => Err(CliError::Usage(format!("unknown manifest sub: {}", u))),
    }
}

fn validate(args: &[String]) -> CliResult {
    let path = args.first().ok_or_else(|| CliError::Usage("manifest validate <path>".into()))?;
    let raw = std::fs::read_to_string(path).map_err(|e| CliError::Io(format!("read {}: {}", path, e)))?;
    let mut have_name = false; let mut have_version = false;
    for line in raw.lines() {
        let l = line.trim();
        if l.starts_with("name ")    || l.starts_with("name=")    { have_name = true; }
        if l.starts_with("version ") || l.starts_with("version=") { have_version = true; }
    }
    if !have_name || !have_version {
        return Err(CliError::Runtime("missing required fields: name, version".into()));
    }
    println!("ok: {}", path);
    Ok(())
}

fn fmt(args: &[String]) -> CliResult {
    let path = args.first().ok_or_else(|| CliError::Usage("manifest fmt <path>".into()))?;
    let raw = std::fs::read_to_string(path).map_err(|e| CliError::Io(format!("read {}: {}", path, e)))?;
    let mut out = String::with_capacity(raw.len());
    for line in raw.lines() {
        let l = line.trim_end();
        if l.is_empty() { out.push('\n'); continue; }
        if let Some((k, v)) = l.split_once('=') {
            out.push_str(k.trim()); out.push_str(" = "); out.push_str(v.trim());
        } else {
            out.push_str(l);
        }
        out.push('\n');
    }
    print!("{}", out);
    Ok(())
}
