//! `cargo krun` runner.
//!
//! Cargo invokes this binary with the kernel ELF as a positional
//! argument (see `.cargo/config.toml` runner directive).  We then:
//!
//!   1. Build a 32 MiB FAT32 raw disk image with the embedded Limine
//!      UEFI loader, our `limine.conf`, and the kernel ELF copied to
//!      the volume root.
//!   2. Spawn `qemu-system-x86_64` against that image with OVMF UEFI
//!      firmware (auto-located across QEMU 11+ install paths).
//!
//! The whole pipeline runs without external utilities (xorriso,
//! mtools, grub-mkrescue) and produces byte-identical disk images on
//! Windows, Linux and macOS hosts.
//!
//! Environment overrides:
//!
//!   * `WAEASI_OVMF`        explicit path to OVMF firmware
//!   * `WAEASI_DISK_BYTES`  raw image size (≥ 33 MiB)
//!   * `WAEASI_QEMU_ARGS`   extra flags passed verbatim to QEMU

mod disk;
mod limine;
mod qemu;

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: runner <kernel-elf>";

fn main() -> ExitCode {
    let kernel = match std::env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    if !kernel.is_file() {
        eprintln!("error: kernel ELF not found: {}", kernel.display());
        return ExitCode::from(2);
    }

    eprintln!("[runner] building boot image");
    let disk = match disk::build(&kernel) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: failed to build disk image: {e}");
            return ExitCode::from(3);
        }
    };
    eprintln!("[runner] image: {}", disk.display());
    eprintln!("[runner] launching qemu-system-x86_64 (Ctrl-A then X to quit)");

    ExitCode::from(qemu::run(&disk) as u8)
}
