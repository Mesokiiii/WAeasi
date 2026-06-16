//! Global `#[panic_handler]`.
//!
//! Bypasses the normal `serial::write_str` path (which holds a SpinLock)
//! and writes directly to UART via `write_str_raw`, so a panic inside a
//! held lock cannot deadlock the panic itself.
use core::panic::PanicInfo;

use crate::arch;
use crate::drivers::serial;

#[panic_handler]
fn on_panic(info: &PanicInfo) -> ! {
    arch::disable_irq(); // freeze cooperative state machine before reporting

    serial::write_str_raw("\n=== KERNEL PANIC ===\n");

    if let Some(loc) = info.location() {
        let mut buf = [0u8; 256];
        let n = format_into(&mut buf, format_args!(
            "  at {}:{}:{}\n", loc.file(), loc.line(), loc.column()
        ));
        serial::write_str_raw(core::str::from_utf8(&buf[..n]).unwrap_or("?\n"));
    }
    let mut buf = [0u8; 512];
    let n = format_into(&mut buf, format_args!("  msg: {}\n", info.message()));
    serial::write_str_raw(core::str::from_utf8(&buf[..n]).unwrap_or("?\n"));

    loop { arch::halt(); }
}

fn format_into(buf: &mut [u8], args: core::fmt::Arguments) -> usize {
    use core::fmt::Write;
    struct Sink<'a> { buf: &'a mut [u8], len: usize }
    impl<'a> Write for Sink<'a> {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            let take = core::cmp::min(s.len(), self.buf.len() - self.len);
            self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
            self.len += take;
            Ok(())
        }
    }
    let mut sink = Sink { buf, len: 0 };
    let _ = sink.write_fmt(args);
    sink.len
}
