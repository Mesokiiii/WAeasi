//! Tracing severity levels — same ordering as `log::Level` so existing
//! `log::info!` / `log::warn!` calls compose with span context.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Level { Trace = 0, Debug = 1, Info = 2, Warn = 3, Error = 4 }

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "TRACE", Self::Debug => "DEBUG",
            Self::Info  => "INFO",  Self::Warn  => "WARN",
            Self::Error => "ERROR",
        }
    }
}
