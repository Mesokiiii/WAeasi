//! `waeasictl doctor` — diagnostics.  Probes every kernel surface the
//! CLI relies on and prints a green/yellow/red status table.
use crate::color;
use crate::conn::Conn;
use crate::error::CliResult;

pub fn run(_args: &[String]) -> CliResult {
    let checks: &[(&str, fn() -> Result<String, String>)] = &[
        ("admin reachable", check_admin),
        ("version reply",   check_version),
        ("metrics endpoint",check_metrics),
        ("health endpoint", check_health),
    ];

    let mut overall = 0;
    for (name, run) in checks {
        match run() {
            Ok(detail) => {
                println!("{} {:<24} {}",
                    color::paint(color::Color::Green, "✓"),
                    name, detail);
            }
            Err(e) => {
                overall = 1;
                println!("{} {:<24} {}",
                    color::paint(color::Color::Red, "✗"),
                    name, e);
            }
        }
    }
    let _ = overall;
    Ok(())
}

fn check_admin()   -> Result<String, String> {
    Conn::open_default().map(|_| String::from("OK"))
        .map_err(|e| e.to_string())
}
fn check_version() -> Result<String, String> {
    let mut c = Conn::open_default().map_err(|e| e.to_string())?;
    c.write_all(b"VERSION\n").map_err(|e| e.to_string())?;
    let s = c.read_to_string().map_err(|e| e.to_string())?;
    Ok(s.lines().next().unwrap_or("?").to_string())
}
fn check_metrics() -> Result<String, String> {
    let mut c = Conn::open_default().map_err(|e| e.to_string())?;
    c.write_all(b"METRICS\n").map_err(|e| e.to_string())?;
    let s = c.read_to_string().map_err(|e| e.to_string())?;
    Ok(format!("{} bytes", s.len()))
}
fn check_health() -> Result<String, String> {
    let mut c = Conn::open_default().map_err(|e| e.to_string())?;
    c.write_all(b"HEALTH\n").map_err(|e| e.to_string())?;
    let s = c.read_to_string().map_err(|e| e.to_string())?;
    let last = s.lines().last().unwrap_or("?");
    Ok(last.to_string())
}
