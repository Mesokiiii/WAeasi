//! `waeasictl metrics` — Prometheus text exposition.
use crate::conn::Conn;
use crate::error::CliResult;

pub fn run(_args: &[String]) -> CliResult {
    let mut c = Conn::open_default()?;
    c.write_all(b"METRICS\n")?;
    let buf = c.read_to_string()?;
    print!("{}", buf);
    Ok(())
}
