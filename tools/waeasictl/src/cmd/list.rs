//! `waeasictl list` — list running components.
use crate::conn::Conn;
use crate::error::CliResult;
use crate::output;

pub fn run(_args: &[String]) -> CliResult {
    let mut c = Conn::open_default()?;
    c.write_all(b"LIST\n")?;
    let body = c.read_to_string()?;

    let rows: Vec<Vec<String>> = body.lines()
        .filter_map(|l| {
            let cols: Vec<String> = l.split('\t').map(String::from).collect();
            if cols.len() >= 5 { Some(cols) } else { None }
        })
        .collect();
    output::render(&["ID", "NAME", "STATE", "CAPS", "MEM"], &rows, Some(2));
    Ok(())
}
