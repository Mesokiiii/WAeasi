//! `waeasi-init <name>` — scaffold a new Wasm component.
//!
//! Generates:
//!   * `components/<name>/Cargo.toml`
//!   * `components/<name>/src/lib.rs`
//!   * `components/<name>/manifest.toml`
//!
//! The output is a complete, build-able crate that can be added to the
//! workspace and dropped into the kernel via `waeasictl run`.
mod template;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: waeasi-init <component-name>");
        std::process::exit(2);
    }
    let name = &args[0];
    if !valid_name(name) {
        eprintln!("invalid name: must be kebab-case [a-z0-9-]");
        std::process::exit(2);
    }
    match template::scaffold(name) {
        Ok(path) => {
            println!("created components/{}/", name);
            println!("  add to workspace Cargo.toml: \"components/{}\"", name);
            println!("  build: cargo build -p {} --target wasm32-unknown-unknown", name);
            let _ = path;
        }
        Err(e) => { eprintln!("scaffold: {}", e); std::process::exit(1); }
    }
}

fn valid_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c|
        c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
