//! `waeasictl ps` — alias for `list` (Linux-style ergonomics).
use crate::cmd::list;
use crate::error::CliResult;

pub fn run(args: &[String]) -> CliResult { list::run(args) }
