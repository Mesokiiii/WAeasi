//! Typed error hierarchy used across all stages.

use std::fmt;
use std::io;
use std::path::PathBuf;

pub type Result<T> = core::result::Result<T, BuildError>;

#[derive(Debug)]
pub enum BuildError {
    Io { path: PathBuf, source: io::Error },
    Toolchain(String),
    BadComponent(String),
    Manifest(String),
    Signature(String),
    Bundle(String),
    Wizer(String),
    Compose(String),
}

impl BuildError {
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        BuildError::Io { path: path.into(), source }
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildError::Io { path, source } =>
                write!(f, "io error at {}: {}", path.display(), source),
            BuildError::Toolchain(s)   => write!(f, "toolchain: {s}"),
            BuildError::BadComponent(s) => write!(f, "bad component: {s}"),
            BuildError::Manifest(s)    => write!(f, "manifest: {s}"),
            BuildError::Signature(s)   => write!(f, "signature: {s}"),
            BuildError::Bundle(s)      => write!(f, "bundle: {s}"),
            BuildError::Wizer(s)       => write!(f, "wizer: {s}"),
            BuildError::Compose(s)     => write!(f, "compose: {s}"),
        }
    }
}

impl std::error::Error for BuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            BuildError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<io::Error> for BuildError {
    fn from(e: io::Error) -> Self {
        BuildError::Io { path: PathBuf::new(), source: e }
    }
}
