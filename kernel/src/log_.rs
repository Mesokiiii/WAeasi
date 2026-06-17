//! Kernel logger — bridges the `log` facade to a UART-direct backend.
//!
//! The backend writes bytes directly to COM1 (`out 0x3f8, al`) without
//! going through the `serial::write_str` `SpinLock` / LSR-poll path.
//! In QEMU stdio every `out` drains instantly, and on real hardware we
//! accept the very small risk of a dropped byte under back-pressure in
//! exchange for a logger that can never deadlock — most importantly
//! during early boot, where the spinlock-based driver has been observed
//! to hang inside its acquire path.
//!
//! Stack budget per record: **256 bytes** (truncates longer messages).
//! Never allocates, never re-enters.
use log::{Level, LevelFilter, Log, Metadata, Record};

const MSG_BUF: usize = 256;

#[inline(always)]
fn raw_putb(b: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16, in("al") b,
            options(nostack, nomem, preserves_flags),
        );
    }
}

#[inline(always)]
fn raw_puts(s: &str) {
    for &b in s.as_bytes() { raw_putb(b); }
}

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
        raw_puts(buf.as_str());
    }
    fn flush(&self) {}
}

static LOGGER: KernelLogger = KernelLogger;

pub fn init() {
    log::set_logger(&LOGGER).ok();
    log::set_max_level(LevelFilter::Info);
}

/// Stack-allocated `core::fmt::Write` — never allocates.
///
/// Buffer is `MaybeUninit<u8>` so construction is a no-op (no zero-init
/// memset), eliminating one possible early-boot footgun if compiler-
/// builtins memset is itself mid-bring-up.  We only ever read bytes we
/// just wrote, indexed by `len`.
struct FixedFmt<const N: usize> {
    buf: [core::mem::MaybeUninit<u8>; N],
    len: usize,
}

impl<const N: usize> FixedFmt<N> {
    fn new() -> Self {
        Self {
            buf: [const { core::mem::MaybeUninit::uninit() }; N],
            len: 0,
        }
    }
    fn as_str(&self) -> &str {
        // SAFETY: bytes [0..len) were written by `write_str`.
        let init = unsafe {
            core::slice::from_raw_parts(self.buf.as_ptr() as *const u8, self.len)
        };
        core::str::from_utf8(init).unwrap_or("")
    }
}

impl<const N: usize> core::fmt::Write for FixedFmt<N> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let take  = core::cmp::min(bytes.len(), N - self.len);
        // SAFETY: writing initialized bytes into the uninit prefix.
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.buf.as_mut_ptr().add(self.len) as *mut u8,
                take,
            );
        }
        self.len += take;
        Ok(())
    }
}
