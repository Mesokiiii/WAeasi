//! `waeasictl logs [--component N] [-f] [-n N] [-s DURATION]` —
//! component log streaming with `tail -f` semantics.
//!
//! Flags:
//!   * `-f / --follow`  — keep streaming new lines until the user
//!     interrupts (Ctrl-C).
//!   * `-n / --tail N`  — only the last N lines.
//!   * `-s / --since D` — only lines newer than `D` (e.g. `30s`, `5m`).
use crate::conn::Conn;
use crate::error::CliResult;

pub fn run(args: &[String]) -> CliResult {
    let mut component: Option<String> = None;
    let mut follow = false;
    let mut tail:   Option<u32>    = None;
    let mut since:  Option<String> = None;

    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--component"      => component = iter.next().cloned(),
            "-f" | "--follow"  => follow = true,
            "-n" | "--tail"    => tail   = iter.next().and_then(|s| s.parse().ok()),
            "-s" | "--since"   => since  = iter.next().cloned(),
            _ => {}
        }
    }

    let mut cmd = String::from("LOGS");
    if let Some(c) = component { cmd.push(' '); cmd.push_str("c="); cmd.push_str(&c); }
    if follow                  { cmd.push_str(" follow=1"); }
    if let Some(n) = tail      { cmd.push_str(" n=");      cmd.push_str(&n.to_string()); }
    if let Some(s) = since     { cmd.push_str(" since=");  cmd.push_str(&s); }
    cmd.push('\n');

    let mut conn = Conn::open_default()?;
    conn.write_all(cmd.as_bytes())?;
    conn.for_each_line(|l| { println!("{}", l); true })
}
