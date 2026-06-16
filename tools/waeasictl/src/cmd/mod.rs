//! Subcommand registry — every module exposes
//! `pub fn run(args: &[String]) -> CliResult`.
pub mod bench;
pub mod cap;
pub mod completion;
pub mod config;
pub mod debug;
pub mod dmesg;
pub mod doctor;
pub mod events;
pub mod exec;
pub mod health;
pub mod inspect;
pub mod lifecycle;
pub mod list;
pub mod logs;
pub mod manifest;
pub mod metrics;
pub mod port_forward;
pub mod profile;
pub mod ps;
pub mod run;
pub mod top;
pub mod trace;
pub mod version;
pub mod wasm;
