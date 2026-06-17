//! Address → source-location resolver, pure Rust.
//!
//! Wraps the `addr2line::Loader` against the kernel ELF, with an
//! in-process cache: panic dumps frequently repeat addresses (return
//! chain, multiple stubs of the same instruction) and re-walking
//! DWARF per query would dominate the runtime.

use std::collections::HashMap;
use std::path::Path;

use addr2line::Loader;

#[derive(Debug, Clone)]
pub struct Location {
    /// Demangled `module::path::function` (best-effort; empty when unknown).
    pub function: String,
    /// `file.rs:line[:col]` if available, otherwise empty.
    pub file:     String,
}

pub struct Resolver {
    loader: Loader,
    cache:  HashMap<u64, Option<Location>>,
}

impl Resolver {
    pub fn new(kernel: &Path) -> std::io::Result<Self> {
        if !kernel.is_file() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("kernel ELF not found: {}", kernel.display()),
            ));
        }
        let loader = Loader::new(kernel).map_err(|e| std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("addr2line load failed: {e}"),
        ))?;
        Ok(Self { loader, cache: HashMap::new() })
    }

    pub fn resolve(&mut self, addr: u64) -> Option<Location> {
        if let Some(hit) = self.cache.get(&addr) {
            return hit.clone();
        }
        let result = self.lookup(addr);
        self.cache.insert(addr, result.clone());
        result
    }

    fn lookup(&self, addr: u64) -> Option<Location> {
        // Take the *innermost* frame: that's the most specific source
        // line, which is what humans want to see first.  We intentionally
        // do not surface the full inline chain here; the report stays
        // dense and one-line-per-address.
        let mut frames = self.loader.find_frames(addr).ok()?;
        let mut function = String::new();
        let mut file     = String::new();
        while let Ok(Some(frame)) = frames.next() {
            if function.is_empty() {
                function = frame.function
                    .as_ref()
                    .and_then(|f| f.demangle().ok().map(|n| n.into_owned()))
                    .unwrap_or_default();
            }
            if file.is_empty() {
                if let Some(loc) = &frame.location {
                    file = format_loc(loc.file, loc.line, loc.column);
                }
            }
        }
        if function.is_empty() && file.is_empty() {
            // Try the loader's `find_location` as a last resort: it
            // succeeds for some non-DWARF-frame stubs (e.g. raw asm
            // entry points) where `find_frames` returns nothing.
            if let Ok(Some(loc)) = self.loader.find_location(addr) {
                file = format_loc(loc.file, loc.line, loc.column);
            }
        }
        if function.is_empty() && file.is_empty() {
            return None;
        }
        Some(Location { function, file })
    }
}

fn format_loc(file: Option<&str>, line: Option<u32>, col: Option<u32>) -> String {
    let mut s = String::new();
    if let Some(f) = file { s.push_str(short_path(f)); }
    if let Some(l) = line { s.push(':'); s.push_str(&l.to_string()); }
    if let Some(c) = col  { s.push(':'); s.push_str(&c.to_string()); }
    s
}

/// Trim absolute paths to a workspace-relative form when possible —
/// `C:\Users\1\Desktop\WAeasi\kernel\src\foo.rs` → `kernel/src/foo.rs`.
/// Falls back to the original string when no `kernel/` segment is found.
fn short_path(p: &str) -> &str {
    for marker in &["kernel\\src\\", "kernel/src/", "tools\\", "tools/"] {
        if let Some(idx) = p.find(marker) {
            return &p[idx..];
        }
    }
    p
}
