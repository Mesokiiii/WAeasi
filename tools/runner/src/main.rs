//! Tiny host-side utility — invoked by `cargo krun` after a successful
//! kernel build.  It just shells out to QEMU with the right flags.
//!
//! Lives **outside** the kernel target so we can use std here.
use std::process::Command;

fn main() {
    let kernel = std::env::args().nth(1)
        .expect("usage: runner <kernel-elf>");
    let status = Command::new("qemu-system-x86_64")
        .args([
            "-machine", "q35",
            "-m",       "256",
            "-cpu",     "max",
            "-serial",  "stdio",
            "-display", "none",
            "-no-reboot",
            "-kernel",  &kernel,
        ])
        .status()
        .expect("failed to start qemu-system-x86_64");
    std::process::exit(status.code().unwrap_or(1));
}
