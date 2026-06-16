//! `waeasictl trace [--level L] [--target T]` — server-filtered tracing.
use crate::conn::Conn;
use crate::error::CliResult;

pub fn run(args: &[String]) -> CliResult {
    let mut level: Option<String> = None;
    let mut target:Option<String> = None;
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--level"  => level  = iter.next().cloned(),
            "--target" => target = iter.next().cloned(),
            _ => {}
        }
    }
    let mut cmd = String::from("TRACE");
    if let Some(l) = level  { cmd.push(' '); cmd.push_str("level=");  cmd.push_str(&l); }
    if let Some(t) = target { cmd.push(' '); cmd.push_str("target="); cmd.push_str(&t); }
    cmd.push('\n');

    let mut c = Conn::open_default()?;
    c.write_all(cmd.as_bytes())?;
    c.for_each_line(|l| { println!("{}", l); true })
}
