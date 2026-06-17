//! Path-discovery helpers for `runner`.
//!
//! We avoid forcing the user to edit `PATH` by probing the well-known
//! install locations of every external tool we depend on.  All
//! lookups follow the same priority:
//!
//!   1. explicit env override,
//!   2. the executable on `$PATH`,
//!   3. a vendor-specific fallback.

use std::path::PathBuf;
use std::process::Command;

pub fn which(bin: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) { format!("{bin}.exe") } else { bin.into() };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(&exe);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

/// Locate `llvm-objcopy`.  Order: `WAEASI_OBJCOPY`, `$PATH`, rustup
/// sysroot (`<sysroot>/lib/rustlib/<host>/bin/llvm-objcopy[.exe]`).
pub fn objcopy() -> std::io::Result<PathBuf> {
    if let Ok(p) = std::env::var("WAEASI_OBJCOPY") {
        let p = PathBuf::from(p);
        if p.is_file() {
            return Ok(p);
        }
    }
    if let Some(p) = which("llvm-objcopy") {
        return Ok(p);
    }
    if let Some(p) = sysroot_bin("llvm-objcopy") {
        return Ok(p);
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "llvm-objcopy not found. Run \
         `rustup component add llvm-tools-preview`, or set \
         WAEASI_OBJCOPY to its absolute path.",
    ))
}

/// Locate `qemu-system-x86_64`.  Falls back to the standard winget /
/// brew install paths so a fresh checkout works without a `PATH` edit.
pub fn qemu() -> PathBuf {
    if let Ok(p) = std::env::var("WAEASI_QEMU") {
        let p = PathBuf::from(p);
        if p.is_file() { return p; }
    }
    if let Some(p) = which("qemu-system-x86_64") { return p; }
    const CANDIDATES: &[&str] = &[
        r"C:\Program Files\qemu\qemu-system-x86_64.exe",
        r"C:\Program Files (x86)\qemu\qemu-system-x86_64.exe",
        "/opt/homebrew/bin/qemu-system-x86_64",
        "/usr/local/bin/qemu-system-x86_64",
        "/usr/bin/qemu-system-x86_64",
    ];
    CANDIDATES.iter().map(PathBuf::from).find(|p| p.is_file())
        .unwrap_or_else(|| PathBuf::from("qemu-system-x86_64"))
}

fn sysroot_bin(name: &str) -> Option<PathBuf> {
    let out = Command::new(std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into()))
        .arg("--print").arg("sysroot")
        .output().ok()?;
    if !out.status.success() { return None; }
    let sysroot = PathBuf::from(String::from_utf8(out.stdout).ok()?.trim());
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.into() };
    let rustlib = sysroot.join("lib").join("rustlib");
    for entry in std::fs::read_dir(&rustlib).ok()?.flatten() {
        let cand = entry.path().join("bin").join(&exe);
        if cand.is_file() { return Some(cand); }
    }
    None
}
