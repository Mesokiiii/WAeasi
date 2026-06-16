//! `waeasictl debug [--gdb]` — attach to the kernel debug surfaces.
use std::process::Command;

use crate::error::CliResult;

pub fn run(args: &[String]) -> CliResult {
    let want_gdb = args.iter().any(|a| a == "--gdb");
    let serial = std::env::var("WAEASI_SERIAL").unwrap_or_else(|_| "/dev/ttyS0".into());

    if !want_gdb {
        println!("debug serial: {}", serial);
        println!("hint: send `$?#3f` to query stop reason");
        return Ok(());
    }
    let kernel = std::env::var("WAEASI_KERNEL")
        .unwrap_or_else(|_| "target/x86_64-waeasi/release/waeasi".into());

    let _ = Command::new("gdb")
        .args(["-q", &kernel, "-ex", &format!("target remote {}", serial)])
        .status();
    Ok(())
}
