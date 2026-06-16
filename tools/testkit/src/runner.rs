//! Scenario runner — spawns QEMU, captures serial output, asserts all
//! `expect` patterns appear before `timeout`.
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::assertions;
use crate::scenario;

pub fn run_path(path: &str) -> Result<(), String> {
    let s = scenario::load(path)?;
    let mut child = Command::new("qemu-system-x86_64")
        .args(&s.qemu_args)
        .args(["-kernel", &s.kernel,
               "-serial", "stdio",
               "-display", "none",
               "-no-reboot"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("qemu spawn: {}", e))?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let reader = BufReader::new(stdout);

    let deadline = Instant::now() + Duration::from_secs(s.timeout_s);
    let mut tracker = assertions::Tracker::new(s.expect);

    for line in reader.lines() {
        if Instant::now() > deadline { break; }
        let line = match line { Ok(l) => l, Err(_) => break };
        tracker.observe(&line);
        if tracker.satisfied() { let _ = child.kill(); return Ok(()); }
    }

    let _ = child.kill();
    Err(tracker.unsatisfied_summary())
}
