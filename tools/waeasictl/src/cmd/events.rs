//! `waeasictl events [--component N] [--since DURATION]` — live event stream.
use crate::conn::Conn;
use crate::error::CliResult;

pub fn run(args: &[String]) -> CliResult {
    let mut component: Option<String> = None;
    let mut since:     Option<String> = None;
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "--component" => component = iter.next().cloned(),
            "--since"     => since     = iter.next().cloned(),
            _ => {}
        }
    }
    let mut cmd = String::from("EVENTS");
    if let Some(c) = component { cmd.push(' '); cmd.push_str("c="); cmd.push_str(&c); }
    if let Some(s) = since     { cmd.push(' '); cmd.push_str("s="); cmd.push_str(&s); }
    cmd.push('\n');

    let mut c = Conn::open_default()?;
    c.write_all(cmd.as_bytes())?;
    c.for_each_line(|l| { println!("{}", l); true })?;
    Ok(())
}
