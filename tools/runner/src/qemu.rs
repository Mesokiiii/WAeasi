//! QEMU launcher.
//!
//! Spawns `qemu-system-x86_64` with the assembled FAT image as a
//! virtio-blk drive and `edk2-x86_64-code.fd` mounted as the read-only
//! UEFI flash chip (CODE region).  A scratch copy of the variables
//! flash (`edk2-i386-vars.fd`) is created per run inside the build
//! directory so guest writes don't leak into the host firmware image.
//!
//! Cross-host: works identically on Windows / Linux / macOS provided
//! QEMU 8+ is on `PATH`.  Honours `WAEASI_OVMF`, `WAEASI_OVMF_VARS`
//! and `WAEASI_QEMU_ARGS` env overrides for non-default installs.

use std::path::{Path, PathBuf};
use std::process::Command;

const CODE_BASENAMES: &[&str] = &[
    "edk2-x86_64-code.fd",
    "OVMF_CODE.fd",
    "OVMF_CODE.4m.fd",
    "ovmf-x86_64-code.fd",
    "OVMF.fd",
];

const VARS_BASENAMES: &[&str] = &[
    "edk2-i386-vars.fd",
    "OVMF_VARS.fd",
    "OVMF_VARS.4m.fd",
];

/// Spawn QEMU and inherit stdio so the kernel's serial output appears
/// directly in the calling shell.  Returns the QEMU exit code, or a
/// nonzero value when QEMU itself could not be launched.
pub fn run(disk: &Path) -> i32 {
    let code = match resolve_firmware(CODE_BASENAMES, "WAEASI_OVMF") {
        Some(p) => p,
        None => return missing("OVMF code firmware (CODE.fd)"),
    };
    // VARS is optional — code-only mode works on most edk2 builds, but
    // emitting a per-run copy is what real distributions do and keeps
    // the host file pristine.
    let vars_template = resolve_firmware(VARS_BASENAMES, "WAEASI_OVMF_VARS");
    let vars_scratch  = vars_template.as_ref().and_then(|src| {
        copy_scratch_vars(src, disk).ok()
    });

    let drive_disk = format!("format=raw,file={}", disk.display());
    let drive_code = format!(
        "if=pflash,format=raw,readonly=on,file={}", code.display(),
    );
    let drive_vars = vars_scratch.as_ref().map(|p| {
        format!("if=pflash,format=raw,file={}", p.display())
    });

    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.args([
        "-machine",  "q35",
        "-m",        "256",
        // `qemu64,+rdrand,-la57` keeps modern userland-relevant feature
        // bits while disabling 5-level paging (LA57) — the kernel and
        // its linker layout assume canonical 4-level x86_64 paging.
        "-cpu",      "qemu64,+rdrand,-la57",
        "-drive",    drive_code.as_str(),
    ]);
    if let Some(v) = drive_vars.as_deref() {
        cmd.args(["-drive", v]);
    }
    cmd.args([
        "-drive",    drive_disk.as_str(),
        "-serial",   "stdio",
        "-display",  "none",
        "-no-reboot",
        "-no-shutdown",
    ]);
    apply_extra_args(&mut cmd);

    match cmd.status() {
        Ok(s)  => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!(
                "error: failed to spawn qemu-system-x86_64: {e}\n\
                 hint: install QEMU and ensure it is on PATH"
            );
            1
        }
    }
}

fn missing(what: &str) -> i32 {
    eprintln!(
        "error: {what} not found.  Set WAEASI_OVMF (and optionally \
         WAEASI_OVMF_VARS) to the firmware paths.  On Windows the \
         file ships at\n  \
         C:\\Program Files\\qemu\\share\\edk2-x86_64-code.fd"
    );
    2
}

fn resolve_firmware(basenames: &[&str], env_override: &str) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var(env_override) {
        let p = PathBuf::from(explicit);
        if p.is_file() {
            return Some(p);
        }
    }
    for dir in candidate_dirs() {
        for base in basenames {
            let p = dir.join(base);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

fn candidate_dirs() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    if let Some(qemu) = which_first("qemu-system-x86_64") {
        if let Some(parent) = qemu.parent() {
            out.push(parent.join("share"));
            out.push(parent.to_path_buf());
        }
    }
    out.push(PathBuf::from(r"C:\Program Files\qemu\share"));
    out.push(PathBuf::from(r"C:\Program Files (x86)\qemu\share"));
    for d in [
        "/usr/share/qemu",
        "/usr/share/edk2/x64",
        "/usr/share/edk2-ovmf/x64",
        "/usr/share/OVMF",
        "/run/current-system/sw/share/qemu",
        "/opt/homebrew/share/qemu",
        "/usr/local/share/qemu",
    ] {
        out.push(PathBuf::from(d));
    }
    out
}

fn which_first(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let exe_ext = if cfg!(windows) { ".exe" } else { "" };
    for dir in std::env::split_paths(&path) {
        let direct = dir.join(format!("{bin}{exe_ext}"));
        if direct.is_file() {
            return Some(direct);
        }
        if !exe_ext.is_empty() {
            let bare = dir.join(bin);
            if bare.is_file() {
                return Some(bare);
            }
        }
    }
    None
}

fn copy_scratch_vars(src: &Path, disk: &Path) -> std::io::Result<PathBuf> {
    let dst = disk
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("ovmf-vars.fd");
    std::fs::copy(src, &dst)?;
    Ok(dst)
}

fn apply_extra_args(cmd: &mut Command) {
    if let Ok(extra) = std::env::var("WAEASI_QEMU_ARGS") {
        for arg in extra.split_whitespace() {
            cmd.arg(arg);
        }
    }
}
