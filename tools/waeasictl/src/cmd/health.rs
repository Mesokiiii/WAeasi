//! `waeasictl health` — liveness + readiness probes.
use crate::conn::Conn;
use crate::error::{CliError, CliResult};

pub fn run(_args: &[String]) -> CliResult {
    let mut c = Conn::open_default()?;
    c.write_all(b"HEALTH\n")?;
    let buf = c.read_to_string()?;
    print!("{}", buf);
    let last = buf.lines().last().unwrap_or("503");
    if last.contains("200") { Ok(()) }
    else { Err(CliError::Server(format!("readyz: {}", last))) }
}
