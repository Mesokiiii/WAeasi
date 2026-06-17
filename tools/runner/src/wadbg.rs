//! Optional `wadbg` integration for the runner.
//!
//! When `WAEASI_WADBG` is set (truthy) and the workspace has a
//! built `wadbg` binary, the runner pipes QEMU's serial output
//! through `wadbg` so crash blocks get annotated with source
//! locations in real time.
//!
//! When the env var is not set or the binary is missing we fall
//! through to the plain "QEMU writes to stdio directly" mode that
//! has been the default for the rest of this development session.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns `Some(path-to-wadbg)` when the user opted in (or when
/// `WAEASI_WADBG` is set explicitly) AND the binary actually exists.
/// Returns `None` to indicate the runner should not pipe.
pub fn locate(workspace_root: &Path) -> Option<PathBuf> {
    let opt_in = std::env::var("WAEASI_WADBG")
        .map(|v| !matches!(v.as_str(), "" | "0" | "false" | "off"))
        .unwrap_or(false);
    if !opt_in {
        return None;
    }

    // Most common locations: explicit override, then host-target build dir.
    if let Ok(p) = std::env::var("WAEASI_WADBG_BIN") {
        let p = PathBuf::from(p);
        if p.is_file() { return Some(p); }
    }
    let exe = if cfg!(windows) { "wadbg.exe" } else { "wadbg" };
    for profile in ["release", "debug"] {
        let cand = workspace_root.join("target").join(profile).join(exe);
        if cand.is_file() { return Some(cand); }
    }
    eprintln!(
        "[runner] WAEASI_WADBG=1 but wadbg binary not found; \
         build it with `cargo build -p wadbg`"
    );
    None
}

/// Build the `wadbg --kernel <path>` command that the runner will use
/// as the destination of QEMU's serial stdout.
pub fn command(wadbg: &Path, kernel: &Path) -> Command {
    let mut cmd = Command::new(wadbg);
    cmd.arg("--kernel").arg(kernel);
    cmd
}
