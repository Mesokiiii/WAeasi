//! Typed error model for the CLI.
//!
//! Exit-code convention (matches `kubectl`/`docker`):
//!   * 0  — success
//!   * 1  — runtime error (connection refused, server-side error)
//!   * 2  — usage error (bad CLI args)
//!   * 3  — IO error (file not found, permission denied)
//!   * 4  — config error (malformed config file)
use std::fmt;

#[derive(Debug)]
pub enum CliError {
    /// User passed wrong arguments — print usage and exit 2.
    Usage(String),
    /// Could not read/write a local file.
    Io(String),
    /// Could not connect to / talk with the kernel.
    Connection(String),
    /// Server returned an error reply.
    Server(String),
    /// `~/.waeasi/config.toml` malformed.
    Config(String),
    /// Generic runtime failure that doesn't fit elsewhere.
    Runtime(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Usage(_)      => 2,
            CliError::Io(_)         => 3,
            CliError::Connection(_) | CliError::Server(_) | CliError::Runtime(_) => 1,
            CliError::Config(_)     => 4,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::Usage(s)      => write!(f, "usage: {}", s),
            CliError::Io(s)         => write!(f, "io: {}", s),
            CliError::Connection(s) => write!(f, "connection: {}", s),
            CliError::Server(s)     => write!(f, "server: {}", s),
            CliError::Config(s)     => write!(f, "config: {}", s),
            CliError::Runtime(s)    => write!(f, "runtime: {}", s),
        }
    }
}

impl From<std::io::Error> for CliError {
    fn from(e: std::io::Error) -> Self { CliError::Io(e.to_string()) }
}

pub type CliResult<T = ()> = Result<T, CliError>;

/// Top-level error reporter — used by `main()` to convert any
/// `CliResult` into the appropriate process exit code.
pub fn report(r: CliResult<()>) -> i32 {
    match r {
        Ok(_)  => 0,
        Err(e) => {
            eprintln!("{}", e);
            e.exit_code()
        }
    }
}
