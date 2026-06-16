//! `waeasictl profile <id> [--seconds N]` — sample-based CPU profile.
//!
//! Asks the kernel to take `N` samples of the component's program-
//! counter at 1 kHz and emits a flame-graph-style histogram.
use crate::conn::Conn;
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    if args.is_empty() {
        return Err(CliError::Usage("profile <component-id> [--seconds N]".into()));
    }
    let id = &args[0];
    let mut seconds: u32 = 5;
    let mut iter = args[1..].iter();
    while let Some(a) = iter.next() {
        if a == "--seconds" {
            if let Some(v) = iter.next() {
                seconds = v.parse().unwrap_or(5);
            }
        }
    }

    let mut c = Conn::open_default()?;
    c.write_all(format!("PROFILE {} {}\n", id, seconds).as_bytes())?;
    let body = c.read_to_string()?;
    print!("{}", body);
    Ok(())
}
