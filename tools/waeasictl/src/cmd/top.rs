//! `waeasictl top [--interval SEC]` — htop-style live view (clears + redraws).
use std::time::Duration;

use crate::conn::Conn;
use crate::error::CliResult;
use crate::output;

pub fn run(args: &[String]) -> CliResult {
    let mut interval_s: f64 = 1.0;
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == "--interval" {
            if let Some(v) = iter.next() {
                interval_s = v.parse::<f64>().unwrap_or(1.0).max(0.1);
            }
        }
    }
    let interval = Duration::from_secs_f64(interval_s);

    loop {
        let mut c = Conn::open_default()?;
        c.write_all(b"TOP\n")?;
        let body = c.read_to_string()?;

        clear_screen();
        let rows: Vec<Vec<String>> = body.lines()
            .filter_map(|l| {
                let cols: Vec<String> = l.split('\t').map(String::from).collect();
                if cols.len() >= 5 { Some(cols) } else { None }
            })
            .collect();
        output::render(&["ID", "NAME", "CPU%", "MEM_KB", "STATE"], &rows, Some(4));
        std::thread::sleep(interval);
    }
}

fn clear_screen() { print!("\x1b[2J\x1b[H"); }
