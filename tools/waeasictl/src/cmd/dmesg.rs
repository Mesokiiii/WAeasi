//! `waeasictl dmesg` — drain kernel ring buffer.
use crate::conn::Conn;
use crate::error::CliResult;

pub fn run(_args: &[String]) -> CliResult {
    let mut c = Conn::open_default()?;
    c.write_all(b"DMESG\n")?;
    c.for_each_line(|l| { println!("{}", l); true })
}
