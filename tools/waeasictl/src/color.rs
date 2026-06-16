//! ANSI color helpers — auto-disable when stdout is not a TTY.
//!
//! Three modes:
//!   * `Auto`  — color iff `stdout` is a terminal AND `--color` not forced.
//!   * `Always`— always emit ANSI escapes (useful when piping to `less -R`).
//!   * `Never` — strip all colour even if stdout is a TTY.
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode { Auto = 0, Always = 1, Never = 2 }

static MODE: AtomicU8 = AtomicU8::new(Mode::Auto as u8);

pub fn set(mode: Mode) { MODE.store(mode as u8, Ordering::Relaxed); }

#[inline]
pub fn enabled() -> bool {
    match MODE.load(Ordering::Relaxed) {
        x if x == Mode::Always as u8 => true,
        x if x == Mode::Never  as u8 => false,
        _ => is_tty_stdout(),
    }
}

/// `true` iff `stdout` is connected to a TTY.  Falls back to `false` on
/// any platform where detection fails.
#[cfg(target_family = "unix")]
fn is_tty_stdout() -> bool {
    extern "C" { fn isatty(fd: i32) -> i32; }
    unsafe { isatty(1) == 1 }
}

#[cfg(target_family = "windows")]
fn is_tty_stdout() -> bool {
    use std::os::windows::io::AsRawHandle;
    extern "system" {
        fn GetConsoleMode(handle: *mut core::ffi::c_void, mode: *mut u32) -> i32;
    }
    let h = std::io::stdout().as_raw_handle();
    let mut mode: u32 = 0;
    unsafe { GetConsoleMode(h, &mut mode) != 0 }
}

#[cfg(not(any(target_family = "unix", target_family = "windows")))]
fn is_tty_stdout() -> bool { false }

#[derive(Copy, Clone, Debug)]
pub enum Color {
    Reset, Bold, Dim,
    Red, Green, Yellow, Blue, Cyan, Magenta, White,
}

impl Color {
    pub fn code(self) -> &'static str {
        match self {
            Color::Reset  => "\x1b[0m",
            Color::Bold   => "\x1b[1m",
            Color::Dim    => "\x1b[2m",
            Color::Red    => "\x1b[31m",
            Color::Green  => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue   => "\x1b[34m",
            Color::Magenta=> "\x1b[35m",
            Color::Cyan   => "\x1b[36m",
            Color::White  => "\x1b[37m",
        }
    }
}

/// Wrap `text` in `c` and a reset.  No-op if color is disabled.
pub fn paint(c: Color, text: &str) -> String {
    if !enabled() { return String::from(text); }
    format!("{}{}{}", c.code(), text, Color::Reset.code())
}

/// Status-like coloring helper — matches common state strings.
pub fn status(state: &str) -> String {
    let c = match state.to_lowercase().as_str() {
        "running" | "ok" | "ready" | "200"            => Color::Green,
        "starting" | "pending" | "stale"              => Color::Yellow,
        "failed" | "error" | "trapped" | "503" | "5xx"=> Color::Red,
        "stopped" | "closed"                          => Color::Dim,
        _ => return String::from(state),
    };
    paint(c, state)
}
