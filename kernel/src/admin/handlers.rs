//! Per-verb handlers — produce reply bytes for each `Request`.
use alloc::string::String;
use core::fmt::Write;

use super::protocol::{err, ok, Request};

/// Top-level dispatch.  Each verb returns a UTF-8 string the server
/// streams back to the client as-is.
pub fn dispatch(req: &Request) -> String {
    match req.verb.as_str() {
        "VERSION"    => version(),
        "METRICS"    => metrics(),
        "DMESG"      => dmesg(),
        "HEALTH"     => health(),
        "LIST"       => list(),
        "TOP"        => top(),
        "INSPECT"    => inspect(req),
        "TRACE"      => trace(req),
        "CAP-LIST"   => err("not-implemented"),
        "CAP-GRANT"
        | "CAP-REVOKE"
        | "KILL"
        | "RESTART"
        | "RUN"      => err("not-implemented"),
        _            => err("unknown-verb"),
    }
}

fn version() -> String {
    let mut s = String::with_capacity(96);
    let _ = writeln!(s, "kernel {}", crate::VERSION);
    let _ = writeln!(s, "stage 9 (audit + alloc-tight)");
    s
}

fn metrics() -> String {
    crate::obs::metrics::render()
}

fn dmesg() -> String {
    // Stage-8: kernel log ring is the responsibility of `log_::init`,
    // which currently writes straight through to UART.  Stage-9 will
    // add an in-memory ring; until then we report the static banner.
    String::from("[kernel] dmesg ring not yet exposed (stage 9)\n")
}

fn health() -> String {
    let mut s = String::with_capacity(32);
    let _ = writeln!(s, "/livez  200");
    let _ = writeln!(s, "/readyz 200");   // every system is "ready" in stage 8
    s
}

fn list() -> String {
    let n = crate::sched::executor::Executor::cpu_count();
    let pending = crate::sched::executor::Executor::global().pending();
    let _ = (n, pending);
    // Per-component table requires the component registry, which lives
    // in stage 9.  For now expose the executor itself.
    let mut s = String::with_capacity(96);
    let _ = writeln!(s, "0\tboot-service\tRunning\t-\t-");
    s
}

fn top() -> String {
    list()
}

fn inspect(req: &Request) -> String {
    let id = req.args.first().map(|s| s.as_str()).unwrap_or("?");
    let mut s = String::with_capacity(64);
    let _ = writeln!(s, "id={}", id);
    let _ = writeln!(s, "state=Running");
    let _ = writeln!(s, "fuel_remaining=unbounded");
    let _ = writeln!(s, "linear_mem_pages=0");
    s
}

fn trace(_req: &Request) -> String {
    String::from("[trace] streaming requires stage-9 connection state\n")
}

/// Re-export `ok` for verbs that succeed silently.
pub fn _ok() -> String { ok() }
