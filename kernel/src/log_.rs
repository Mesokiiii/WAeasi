//! Kernel logger — bridges the `log` facade to the early serial driver.
//!
//! Stack budget per record: **256 bytes** (was 512).  Long messages
//! get truncated; we never allocate, never block, never re-enter the
//! logger on a full ring.
use log::{Level, LevelFilter, Log, Metadata, Record};

use crate::drivers::serial;

const MSG_BUF: usize = 256;

struct KernelLogger;

impl Log for KernelLogger {
    fn enabled(&self, _: &Metadata) -> bool { true }
    fn log(&self, record: &Record) {
        let lvl = match record.level() {
            Level::Error => "E", Level::Warn => "W",
            Level::Info  => "I", Level::Debug => "D", Level::Trace => "T",
        };
        let mut buf = FixedFmt::<MSG_BUF>::new();
        let _ = core::fmt::write(
            &mut buf,
            format_args!("[{}] {} {}\n", lvl, record.target(), record.args()),
        );
        serial::write_str(buf.as_str());
    }
    fn flush(&self) {}
}

static LOGGER: KernelLogger = KernelLogger;

pub fn init() {
    log::set_logger(&LOGGER).ok();
    log::set_max_level(LevelFilter::Info);
}

/// Stack-allocated `core::fmt::Write` — never allocates.
struct FixedFmt<const N: usize> { buf: [u8; N], len: usize }

impl<const N: usize> FixedFmt<N> {
    fn new() -> Self { Self { buf: [0; N], len: 0 } }
    fn as_str(&self) -> &str { core::str::from_utf8(&self.buf[..self.len]).unwrap_or("") }
}

impl<const N: usize> core::fmt::Write for FixedFmt<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let take  = core::cmp::min(bytes.len(), N - self.len);
        self.buf[self.len..self.len + take].copy_from_slice(&bytes[..take]);
        self.len += take;
        Ok(())
    }
}
