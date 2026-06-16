//! `waeasictl version` — kernel version + build features.
use crate::conn::Conn;
use crate::error::CliResult;

pub fn run(_args: &[String]) -> CliResult {
    let mut c = Conn::open_default()?;
    c.write_all(b"VERSION\n")?;
    print!("{}", c.read_to_string()?);
    Ok(())
}
