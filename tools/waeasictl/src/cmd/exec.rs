//! `waeasictl exec <id> <export> [args...]` — RPC into a component.
//!
//! Calls a named component export with the given arguments and prints
//! the return value(s) line by line.
use crate::conn::Conn;
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    if args.len() < 2 {
        return Err(CliError::Usage("exec <component-id> <export> [args...]".into()));
    }
    let id     = &args[0];
    let export = &args[1];
    let xargs: Vec<String> = args[2..].iter().cloned().collect();

    let mut cmd = format!("EXEC {} {}", id, export);
    for a in &xargs { cmd.push(' '); cmd.push_str(a); }
    cmd.push('\n');

    let mut c = Conn::open_default()?;
    c.write_all(cmd.as_bytes())?;
    let reply = c.read_to_string()?;
    print!("{}", reply);
    Ok(())
}
