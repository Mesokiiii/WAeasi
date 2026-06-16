//! `waeasictl kill <id>` and `restart <id>`.
use crate::conn::Conn;
use crate::error::{CliError, CliResult};

pub fn kill(args: &[String])    -> CliResult { send_simple("KILL",    args, "kill") }
pub fn restart(args: &[String]) -> CliResult { send_simple("RESTART", args, "restart") }

fn send_simple(verb: &str, args: &[String], usage: &str) -> CliResult {
    let id = args.first().ok_or_else(|| CliError::Usage(format!("{} <id>", usage)))?;
    let mut c = Conn::open_default()?;
    c.write_all(format!("{} {}\n", verb, id).as_bytes())?;
    print!("{}", c.read_to_string()?);
    Ok(())
}
