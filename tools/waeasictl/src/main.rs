//! `waeasictl` — host-side CLI for WAeasi.
//!
//! Global flags (must precede the subcommand):
//!
//!   `--server <host:port>`  Admin endpoint (default 127.0.0.1:9300).
//!   `--output {table,wide,json,yaml}`  Output format (alias `-o`).
//!   `--color {auto,always,never}`      ANSI color policy.
//!   `-v / --verbose`        Verbose logging on stderr.
//!   `-q / --quiet`          Suppress informational stderr.
//!   `--json`                Shortcut for `--output json`.
//!
//! Settings are read from `~/.waeasi/config.toml`; CLI flags override
//! env vars override the config file override built-in defaults.
mod cmd;
mod color;
mod config;
mod conn;
mod error;
mod output;

use error::{CliError, CliResult, report};

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let (flags, args) = split_global_flags(&raw);
    let rc = match apply_global_flags(&flags) {
        Ok(())  => report(dispatch(&args)),
        Err(e)  => report(Err(e)),
    };
    std::process::exit(rc);
}

fn dispatch(args: &[String]) -> CliResult {
    let sub = args.first().map(|s| s.as_str())
        .ok_or_else(|| { print_usage(); CliError::Usage("missing subcommand".into()) })?;
    let rest = &args[1..];
    match sub {
        "list"        | "ls"            => cmd::list::run(rest),
        "ps"                            => cmd::ps::run(rest),
        "run"         | "deploy"        => cmd::run::run(rest),
        "inspect"     | "describe"      => cmd::inspect::run(rest),
        "top"                           => cmd::top::run(rest),
        "kill"                          => cmd::lifecycle::kill(rest),
        "restart"                       => cmd::lifecycle::restart(rest),
        "logs"                          => cmd::logs::run(rest),
        "tail"                          => cmd::logs::run(&prepend("-f", rest)),
        "events"                        => cmd::events::run(rest),
        "trace"                         => cmd::trace::run(rest),
        "metrics"                       => cmd::metrics::run(rest),
        "dmesg"                         => cmd::dmesg::run(rest),
        "health"                        => cmd::health::run(rest),
        "version"                       => cmd::version::run(rest),
        "cap"                           => cmd::cap::run(rest),
        "wasm"                          => cmd::wasm::run(rest),
        "manifest"                      => cmd::manifest::run(rest),
        "debug"                         => cmd::debug::run(rest),
        "completion"                    => cmd::completion::run(rest),
        "config"                        => cmd::config::run(rest),
        "doctor"                        => cmd::doctor::run(rest),
        "bench"                         => cmd::bench::run(rest),
        "exec"                          => cmd::exec::run(rest),
        "port-forward" | "pf"           => cmd::port_forward::run(rest),
        "profile"                       => cmd::profile::run(rest),
        "--help" | "-h" | "help"        => { print_usage(); Ok(()) }
        u => Err(CliError::Usage(format!("unknown subcommand '{}'", u))),
    }
}

fn split_global_flags(args: &[String]) -> (Vec<String>, Vec<String>) {
    let mut flags = Vec::new();
    let mut rest  = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--json" | "-v" | "--verbose" | "-q" | "--quiet" => flags.push(a.clone()),
            "--server" | "--output" | "-o" | "--color" => {
                flags.push(a.clone()); i += 1;
                if i < args.len() { flags.push(args[i].clone()); }
            }
            _ => { rest.extend_from_slice(&args[i..]); break; }
        }
        i += 1;
    }
    (flags, rest)
}

fn apply_global_flags(flags: &[String]) -> CliResult {
    let mut iter = flags.iter();
    while let Some(f) = iter.next() {
        match f.as_str() {
            "--json" => output::set(output::Format::Json),
            "--server" => if let Some(s) = iter.next() {
                std::env::set_var("WAEASI_ADMIN", s);
            },
            "--output" | "-o" => match iter.next().map(|s| s.as_str()) {
                Some("table") => output::set(output::Format::Table),
                Some("wide")  => output::set(output::Format::Wide),
                Some("json")  => output::set(output::Format::Json),
                Some("yaml")  => output::set(output::Format::Yaml),
                Some(u) => return Err(CliError::Usage(
                    format!("--output ∈ {{table,wide,json,yaml}}, got '{}'", u))),
                None => return Err(CliError::Usage("--output needs value".into())),
            },
            "--color" => match iter.next().map(|s| s.as_str()) {
                Some("auto")   => color::set(color::Mode::Auto),
                Some("always") => color::set(color::Mode::Always),
                Some("never")  => color::set(color::Mode::Never),
                _ => return Err(CliError::Usage("--color ∈ {auto,always,never}".into())),
            },
            "-v" | "--verbose" => std::env::set_var("WAEASI_VERBOSE", "1"),
            "-q" | "--quiet"   => std::env::set_var("WAEASI_VERBOSE", "0"),
            _ => {}
        }
    }
    // Apply config file defaults (after explicit flags so flag overrides).
    if let Ok(cfg) = config::load() {
        if std::env::var("WAEASI_ADMIN").is_err() {
            std::env::set_var("WAEASI_ADMIN", &cfg.server);
        }
        match cfg.output {
            config::Output::Table => {}, // default
            config::Output::Wide  => if !flags.iter().any(|f| f == "--output" || f == "-o") {
                output::set(output::Format::Wide);
            },
            config::Output::Json  => if !flags.iter().any(|f| f == "--json" || f == "--output" || f == "-o") {
                output::set(output::Format::Json);
            },
            config::Output::Yaml  => if !flags.iter().any(|f| f == "--output" || f == "-o") {
                output::set(output::Format::Yaml);
            },
        }
        color::set(cfg.color);
    }
    Ok(())
}

fn prepend(flag: &str, args: &[String]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len() + 1);
    out.push(String::from(flag));
    out.extend_from_slice(args);
    out
}

fn print_usage() {
    println!("waeasictl [GLOBAL FLAGS] <subcommand> [args]");
    println!();
    println!("Global flags:");
    println!("  --server H:P            admin endpoint");
    println!("  -o, --output FMT        table|wide|json|yaml");
    println!("  --color MODE            auto|always|never");
    println!("  --json                  shortcut for --output json");
    println!("  -v, --verbose           verbose logging");
    println!("  -q, --quiet             suppress informational output");
    println!();
    println!("Subcommands (Linux/k8s-style aliases in parens):");
    println!("  list (ls)        run (deploy)    inspect (describe)");
    println!("  ps               top             logs    tail");
    println!("  events           trace           metrics dmesg");
    println!("  health           version         kill    restart");
    println!("  cap              wasm            manifest debug");
    println!("  completion       config          doctor");
    println!("  bench            exec            port-forward (pf)");
    println!("  profile");
}
