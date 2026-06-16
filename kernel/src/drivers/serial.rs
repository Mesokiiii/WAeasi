//! 16550 UART driver — the very first subsystem brought up so panic
//! messages survive even if the rest of the kernel is unhealthy.
//!
//! **Panic-safety contract**: `write_str_raw` must work even if the lock
//! is poisoned/held — it bypasses the `SpinLock` and writes directly to
//! the UART.  Use it only from `panic_handler` and #DF.
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};
use uart_16550::SerialPort;

use crate::sync::SpinLock;

const COM1: u16 = 0x3F8;

static SERIAL: SpinLock<Option<SerialPort>> = SpinLock::new(None);
static READY:  AtomicBool                   = AtomicBool::new(false);

pub fn init() {
    let mut port = unsafe { SerialPort::new(COM1) };
    port.init();
    *SERIAL.lock() = Some(port);
    READY.store(true, Ordering::Release);
}

/// Normal path — IRQ-safe via `SpinLock`.
pub fn write_str(s: &str) {
    if !READY.load(Ordering::Acquire) { return; }
    if let Some(port) = SERIAL.lock().as_mut() {
        let _ = port.write_str(s);
    }
}

/// Panic / double-fault path — bypasses the lock entirely.  We accept the
/// risk of garbled output in exchange for never deadlocking on a held lock.
pub fn write_str_raw(s: &str) {
    let mut port = unsafe { SerialPort::new(COM1) };
    let _ = port.write_str(s);
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    if !READY.load(Ordering::Acquire) { return; }
    if let Some(port) = SERIAL.lock().as_mut() {
        let _ = port.write_fmt(args);
    }
}

#[macro_export]
macro_rules! sprint {
    ($($arg:tt)*) => ($crate::drivers::serial::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! sprintln {
    ()              => ($crate::sprint!("\n"));
    ($($arg:tt)*)   => ($crate::sprint!("{}\n", format_args!($($arg)*)));
}
