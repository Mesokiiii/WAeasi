//! `waeasictl bench [-c CONNS] [-n REQS] [-d DURATION] <component>` —
//! built-in load tester.  Mirrors the `wrk` / `hey` style.
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::conn::Conn;
use crate::error::{CliError, CliResult};

#[derive(Debug)]
struct Opts {
    component: String,
    connections: u32,
    requests:    u64,
    duration_s:  Option<u64>,
}

pub fn run(args: &[String]) -> CliResult {
    let opts = parse(args)?;

    let total_ok   = Arc::new(AtomicU64::new(0));
    let total_fail = Arc::new(AtomicU64::new(0));

    let started = Instant::now();
    let deadline = opts.duration_s.map(|s| started + Duration::from_secs(s));
    let per_thread_reqs = opts.requests / opts.connections.max(1) as u64;

    let mut handles = Vec::with_capacity(opts.connections as usize);
    for _ in 0..opts.connections {
        let c = opts.component.clone();
        let ok = total_ok.clone();
        let fail = total_fail.clone();
        handles.push(thread::spawn(move || {
            let mut sent = 0u64;
            loop {
                if let Some(d) = deadline { if Instant::now() >= d { break; } }
                if opts.duration_s.is_none() && sent >= per_thread_reqs { break; }
                match probe(&c) {
                    Ok(_)  => ok.fetch_add(1, Ordering::Relaxed),
                    Err(_) => fail.fetch_add(1, Ordering::Relaxed),
                };
                sent += 1;
            }
        }));
    }
    for h in handles { let _ = h.join(); }

    let ok = total_ok.load(Ordering::Relaxed);
    let fail = total_fail.load(Ordering::Relaxed);
    let elapsed = started.elapsed().as_secs_f64();
    let rps = if elapsed > 0.0 { (ok + fail) as f64 / elapsed } else { 0.0 };

    println!("benchmark results:");
    println!("  component  : {}", opts.component);
    println!("  duration   : {:.2}s", elapsed);
    println!("  connections: {}", opts.connections);
    println!("  ok         : {}", ok);
    println!("  fail       : {}", fail);
    println!("  rps        : {:.0}", rps);
    Ok(())
}

fn probe(component: &str) -> CliResult {
    let mut c = Conn::open_default()?;
    c.write_all(format!("PROBE {}\n", component).as_bytes())?;
    let _ = c.read_to_string()?;
    Ok(())
}

fn parse(args: &[String]) -> CliResult<Opts> {
    let mut o = Opts {
        component: String::new(),
        connections: 16, requests: 1000, duration_s: None,
    };
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        match a.as_str() {
            "-c" => o.connections = iter.next().and_then(|s| s.parse().ok()).unwrap_or(16),
            "-n" => o.requests   = iter.next().and_then(|s| s.parse().ok()).unwrap_or(1000),
            "-d" => o.duration_s = iter.next().and_then(|s| s.parse().ok()),
            x if !x.starts_with('-') => o.component = x.to_string(),
            _ => {}
        }
    }
    if o.component.is_empty() {
        return Err(CliError::Usage("bench [-c N] [-n N] [-d SEC] <component>".into()));
    }
    Ok(o)
}
