//! `waeasictl config <show|set|edit>` — manage `~/.waeasi/config.toml`.
use crate::config::{self, Config};
use crate::error::{CliError, CliResult};

pub fn run(args: &[String]) -> CliResult {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("show");
    match sub {
        "show" => show(),
        "set"  => set(&args[1..]),
        "edit" => edit(),
        "path" => { println!("{}", config::path().display()); Ok(()) }
        u => Err(CliError::Usage(format!("config {{show|set|edit|path}} (got '{}')", u))),
    }
}

fn show() -> CliResult {
    let cfg = config::load()?;
    println!("server  = \"{}\"", cfg.server);
    println!("output  = {:?}", cfg.output);
    println!("color   = {:?}", cfg.color);
    println!("verbose = {}",   cfg.verbose);
    Ok(())
}

fn set(args: &[String]) -> CliResult {
    if args.len() < 2 {
        return Err(CliError::Usage("config set <key> <value>".into()));
    }
    let mut cfg: Config = config::load().unwrap_or_default();
    let key = &args[0]; let val = &args[1];
    match key.as_str() {
        "server" => cfg.server = val.clone(),
        "output" => match val.as_str() {
            "table" => cfg.output = config::Output::Table,
            "wide"  => cfg.output = config::Output::Wide,
            "json"  => cfg.output = config::Output::Json,
            "yaml"  => cfg.output = config::Output::Yaml,
            _ => return Err(CliError::Usage("output ∈ {table,wide,json,yaml}".into())),
        },
        "color" => match val.as_str() {
            "auto"   => cfg.color = crate::color::Mode::Auto,
            "always" => cfg.color = crate::color::Mode::Always,
            "never"  => cfg.color = crate::color::Mode::Never,
            _ => return Err(CliError::Usage("color ∈ {auto,always,never}".into())),
        },
        "verbose" => cfg.verbose = val == "true" || val == "1",
        _ => return Err(CliError::Usage(format!("unknown key '{}'", key))),
    }
    config::save(&cfg)?;
    println!("saved {}", config::path().display());
    Ok(())
}

fn edit() -> CliResult {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| String::from("notepad"));
    let path = config::path();
    if !path.exists() { config::save(&Config::default())?; }
    let _ = std::process::Command::new(&editor).arg(&path).status();
    Ok(())
}
