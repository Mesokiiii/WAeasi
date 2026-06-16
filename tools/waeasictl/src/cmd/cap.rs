//! `waeasictl cap grant|revoke|list <id> [RIGHTS...]`.
use crate::conn::Conn;
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    let sub = args.first().map(|s| s.as_str())
        .ok_or_else(|| CliError::Usage("cap <grant|revoke|list> <id> [RIGHTS...]".into()))?;
    match sub {
        "grant"  => mutate("CAP-GRANT",  &args[1..]),
        "revoke" => mutate("CAP-REVOKE", &args[1..]),
        "list"   => list(&args[1..]),
        u        => Err(CliError::Usage(format!("unknown cap sub: {}", u))),
    }
}

fn mutate(verb: &str, args: &[String]) -> CliResult {
    if args.len() < 2 {
        return Err(CliError::Usage(format!("cap {} <id> RIGHT...",
            verb.split('-').last().unwrap_or(verb).to_lowercase())));
    }
    let id = &args[0];
    let rights = args[1..].join(",");
    let mut c = Conn::open_default()?;
    c.write_all(format!("{} {} {}\n", verb, id, rights).as_bytes())?;
    print!("{}", c.read_to_string()?);
    Ok(())
}

fn list(args: &[String]) -> CliResult {
    let id = args.first().ok_or_else(|| CliError::Usage("cap list <id>".into()))?;
    let mut c = Conn::open_default()?;
    c.write_all(format!("CAP-LIST {}\n", id).as_bytes())?;
    print!("{}", c.read_to_string()?);
    Ok(())
}
