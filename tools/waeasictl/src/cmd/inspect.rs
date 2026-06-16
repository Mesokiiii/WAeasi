//! `waeasictl inspect <id>` — detailed component snapshot.
use crate::conn::Conn;
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    let id = args.first().ok_or_else(|| CliError::Usage("inspect <id>".into()))?;
    let mut c = Conn::open_default()?;
    c.write_all(format!("INSPECT {}\n", id).as_bytes())?;
    print!("{}", c.read_to_string()?);
    Ok(())
}
