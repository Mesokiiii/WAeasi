//! `testkit` — boots the WAeasi kernel under QEMU and runs a series of
//! scenarios against it, asserting expected serial output / metrics.
//!
//! Tests live as `.test.toml` files; each scenario lists `expect:`
//! patterns that must appear in the kernel's serial output before the
//! `timeout` elapses.
mod assertions;
mod runner;
mod scenario;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("testkit <scenario.test.toml> [more.test.toml ...]");
        std::process::exit(2);
    }
    let mut failures = 0;
    for path in &args[1..] {
        match runner::run_path(path) {
            Ok(()) => println!("✓ {}", path),
            Err(e) => { println!("✗ {}: {}", path, e); failures += 1; }
        }
    }
    if failures > 0 {
        eprintln!("{} scenario(s) failed", failures);
        std::process::exit(1);
    }
}
