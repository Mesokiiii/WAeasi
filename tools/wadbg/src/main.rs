//! `wadbg` — WAeasi kernel crash-dump debugger (host side).
//!
//! Three modes:
//!
//!   * **stream** (default) — read kernel serial output on stdin,
//!     forward it verbatim, and on every crash block append a
//!     decoded report (vector name, error-code bits, source location
//!     of `RIP`, optional source location of `CR2`, and any `at …`
//!     return addresses found in the trailing backtrace lines).
//!
//!   * **resolve** — one-shot lookup of a single hex address against
//!     the kernel ELF.  Useful for ad-hoc spelunking from a shell.
//!
//!   * **int-log** — parse a `qemu -d int` log and print a one-line
//!     summary per exception: `vec, ec_decoded, RIP@source`.
//!
//! No third-party crates: the resolver shells out to `llvm-addr2line`
//! (which `rustup component add llvm-tools-preview` already installs
//! and which `tools/runner` already locates inside the rustup sysroot).
mod decode;
mod intlog;
mod parse;
mod pretty;
mod resolve;

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "\
wadbg — WAeasi crash-dump debugger

USAGE:
  wadbg --kernel <kernel.elf>                         # stream stdin
  wadbg --kernel <kernel.elf> --resolve <hex-addr>    # one-shot
  wadbg --kernel <kernel.elf> --int-log <file>        # decode -d int log

OPTIONS:
  --kernel <path>     Path to the unstripped kernel ELF (for symbols).
  --resolve <addr>    Resolve one hex address (with or without 0x).
  --int-log <path>    Decode a `-d int` log produced by QEMU.
  --no-color          Disable ANSI colour output.
  --help              Show this message.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprint!("{USAGE}");
        return ExitCode::from(0);
    }

    let mut kernel: Option<PathBuf> = None;
    let mut resolve_one: Option<String> = None;
    let mut int_log: Option<PathBuf> = None;
    let mut color = pretty::Color::Auto;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--kernel"   => { kernel = Some(PathBuf::from(arg(&args, i + 1, "--kernel"))); i += 2; }
            "--resolve"  => { resolve_one = Some(arg(&args, i + 1, "--resolve")); i += 2; }
            "--int-log"  => { int_log = Some(PathBuf::from(arg(&args, i + 1, "--int-log"))); i += 2; }
            "--no-color" => { color = pretty::Color::Off; i += 1; }
            other        => { eprintln!("wadbg: unknown argument {other:?}\n{USAGE}"); return ExitCode::from(2); }
        }
    }

    let Some(kernel) = kernel else {
        eprintln!("wadbg: --kernel <path> is required\n{USAGE}");
        return ExitCode::from(2);
    };

    let mut resolver = match resolve::Resolver::new(&kernel) {
        Ok(r)  => r,
        Err(e) => { eprintln!("wadbg: cannot create resolver: {e}"); return ExitCode::from(1); }
    };
    let style = pretty::Style::for_stream(color, std::io::stderr().lock());

    if let Some(addr) = resolve_one {
        return resolve_one_address(&mut resolver, &style, &addr);
    }
    if let Some(p) = int_log {
        return intlog::run(&mut resolver, &style, &p);
    }
    stream(&mut resolver, &style)
}

fn arg(args: &[String], i: usize, name: &str) -> String {
    args.get(i).cloned().unwrap_or_else(|| {
        eprintln!("wadbg: {name} requires a value");
        std::process::exit(2);
    })
}

fn resolve_one_address(r: &mut resolve::Resolver, st: &pretty::Style, hex: &str) -> ExitCode {
    let trimmed = hex.trim_start_matches("0x").trim_start_matches("0X");
    let Ok(addr) = u64::from_str_radix(trimmed, 16) else {
        eprintln!("wadbg: not a hex address: {hex}");
        return ExitCode::from(2);
    };
    match r.resolve(addr) {
        Some(loc) => { st.print_resolved(addr, &loc); ExitCode::from(0) }
        None      => { st.print_unresolved(addr);   ExitCode::from(1) }
    }
}

fn stream(r: &mut resolve::Resolver, _st: &pretty::Style) -> ExitCode {
    let stdin  = io::stdin();
    let mut out = io::stdout().lock();
    let mut p = parse::Parser::new();
    for line in stdin.lock().lines().map_while(Result::ok) {
        // Forward every line verbatim so the operator still sees the
        // raw kernel output unchanged.
        let _ = writeln!(out, "{line}");
        if let Some(crash) = p.feed(&line) {
            let report = pretty::format_crash(&crash, r);
            let _ = writeln!(out, "{report}");
            let _ = out.flush();
        }
    }
    ExitCode::from(0)
}
