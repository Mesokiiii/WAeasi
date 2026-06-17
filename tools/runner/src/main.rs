//! `cargo krun` runner.
//!
//! Cargo invokes this binary with the kernel ELF as a positional
//! argument (see `.cargo/config.toml`).  The kernel publishes a
//! Multiboot1 header so QEMU can load it directly via `-kernel`.
//!
//! Environment overrides:
//!
//!   * `WAEASI_QEMU`         path to the QEMU executable
//!   * `WAEASI_QEMU_ARGS`    extra arguments appended verbatim
//!   * `WAEASI_QEMU_MEMORY`  guest RAM in MiB (default 256)
//!   * `WAEASI_QEMU_CPU`     `-cpu` value (default `qemu64,+rdrand`)
//!   * `WAEASI_OBJCOPY`      override `llvm-objcopy` path
//!   * `WAEASI_WADBG`        when truthy, pipe serial stdout through
//!                           `wadbg --kernel <ELF>` for live crash
//!                           annotation (see `tools/wadbg`).

mod locate;
mod wadbg;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

const USAGE: &str = "usage: runner <kernel-elf>";

fn main() -> ExitCode {
    let kernel = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => { eprintln!("{USAGE}"); return ExitCode::from(2); }
    };
    if !kernel.is_file() {
        eprintln!("error: kernel ELF not found: {}", kernel.display());
        return ExitCode::from(2);
    }

    let qemu = locate::qemu();
    let mem  = std::env::var("WAEASI_QEMU_MEMORY").unwrap_or_else(|_| "256".into());
    let cpu  = std::env::var("WAEASI_QEMU_CPU").unwrap_or_else(|_| "qemu64,+rdrand".into());

    eprintln!("[runner] launching {} (Ctrl-A then X to quit)", qemu.display());

    let elf32 = match repackage_as_elf32(&kernel) {
        Ok(p)  => p,
        Err(e) => { eprintln!("error: ELF32 repackage failed: {e}"); return ExitCode::from(3); }
    };

    let workspace = workspace_root();
    let dbg = wadbg::locate(&workspace);

    let mut qemu_cmd = Command::new(&qemu);
    qemu_cmd.args([
        "-machine",   "q35",
        "-m",         mem.as_str(),
        "-cpu",       cpu.as_str(),
        "-serial",    "stdio",
        "-display",   "none",
        "-no-reboot",
        "-kernel",
    ]);
    qemu_cmd.arg(&elf32);
    if let Ok(extra) = std::env::var("WAEASI_QEMU_ARGS") {
        for arg in extra.split_whitespace() {
            qemu_cmd.arg(arg);
        }
    }

    let exit = if let Some(wadbg_bin) = dbg {
        eprintln!("[runner] piping serial through {}", wadbg_bin.display());
        run_with_wadbg(&mut qemu_cmd, &wadbg_bin, &kernel)
    } else {
        run_plain(&mut qemu_cmd, &qemu)
    };
    ExitCode::from(exit as u8)
}

fn run_plain(qemu_cmd: &mut Command, qemu: &Path) -> i32 {
    match qemu_cmd.status() {
        Ok(s)  => s.code().unwrap_or(1),
        Err(e) => {
            eprintln!(
                "error: failed to spawn {}: {e}\n\
                 hint: install QEMU and ensure it is on PATH \
                 (or set WAEASI_QEMU to its absolute path)",
                qemu.display(),
            );
            1
        }
    }
}

fn run_with_wadbg(qemu_cmd: &mut Command, wadbg_bin: &Path, kernel: &Path) -> i32 {
    let mut wadbg = match wadbg::command(wadbg_bin, kernel)
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(c)  => c,
        Err(e) => { eprintln!("[runner] wadbg failed to start: {e}; falling back"); return run_plain(qemu_cmd, &PathBuf::from("qemu")); }
    };
    let pipe = wadbg.stdin.take().expect("piped stdin");
    qemu_cmd.stdout(pipe).stderr(Stdio::inherit());
    let qemu_status = qemu_cmd.status();
    let _ = wadbg.wait();
    match qemu_status {
        Ok(s)  => s.code().unwrap_or(1),
        Err(e) => { eprintln!("[runner] qemu exited abnormally: {e}"); 1 }
    }
}

/// Convert a 64-bit ELF kernel to a 32-bit ELF wrapper using
/// `llvm-objcopy --output-target=elf32-i386`.  QEMU's `-kernel`
/// loader accepts only ELF32 headers; the actual machine code can
/// freely mix 32-bit and 64-bit (the trampoline uses both).
fn repackage_as_elf32(input: &Path) -> std::io::Result<PathBuf> {
    let bin = locate::objcopy()?;
    let out = input.with_extension("elf32");
    let status = Command::new(&bin)
        .args(["-O", "elf32-i386", "--strip-unneeded"])
        .arg(input).arg(&out).status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "{} exited with {:?}", bin.display(), status.code(),
        )));
    }
    Ok(out)
}

fn workspace_root() -> PathBuf {
    // The runner is launched by `cargo krun`, which sets `CARGO_MANIFEST_DIR`
    // to `tools/runner` — climb two levels.  Falls back to the current
    // working directory.
    if let Ok(d) = std::env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(d);
        if let Some(workspace) = p.ancestors().nth(2) {
            return workspace.to_path_buf();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
