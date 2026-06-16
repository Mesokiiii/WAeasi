//! Scenario file parser — minimal TOML-ish format.
//!
//! ```toml
//! kernel  = "target/x86_64-waeasi/release/waeasi"
//! timeout = 10                  # seconds
//! qemu_args = ["-machine", "q35", "-m", "256"]
//!
//! [[expect]]
//! pattern = "kernel v0.1.0 booting"
//!
//! [[expect]]
//! pattern = "executor CPU 0 entering main loop"
//! ```
use std::fs;

#[derive(Debug, Default)]
pub struct Scenario {
    pub kernel:    String,
    pub timeout_s: u64,
    pub qemu_args: Vec<String>,
    pub expect:    Vec<String>,
}

pub fn load(path: &str) -> Result<Scenario, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("read: {}", e))?;
    let mut s = Scenario { timeout_s: 10, ..Scenario::default() };
    let mut in_expect = false;

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        if line.starts_with("[[expect]]") { in_expect = true; continue; }
        if line.starts_with('[')          { in_expect = false; continue; }

        let (k, v) = match line.split_once('=') {
            Some(p) => p,
            None    => continue,
        };
        let key = k.trim();
        let val = v.trim();
        match (key, in_expect) {
            ("kernel", _)    => s.kernel    = strip_str(val),
            ("timeout", _)   => s.timeout_s = val.parse().unwrap_or(10),
            ("qemu_args", _) => s.qemu_args = parse_str_array(val),
            ("pattern", true)=> s.expect.push(strip_str(val)),
            _ => {}
        }
    }
    if s.kernel.is_empty() { return Err("missing 'kernel' field".into()); }
    Ok(s)
}

fn strip_str(v: &str) -> String {
    let v = v.trim();
    if v.starts_with('"') && v.ends_with('"') { v[1..v.len()-1].into() } else { v.into() }
}

fn parse_str_array(v: &str) -> Vec<String> {
    let v = v.trim();
    if !v.starts_with('[') || !v.ends_with(']') { return Vec::new(); }
    v[1..v.len()-1].split(',').map(|t| strip_str(t.trim())).collect()
}
