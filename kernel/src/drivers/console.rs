//! VGA text-mode console (B8000h) — used as the default `stdout` when the
//! kernel is not running headless.  Stage 2 will swap to a framebuffer.
//!
//! The MMIO is touched through `core::ptr::write_volatile` directly — the
//! `volatile` crate's `Volatile<T>` wrapper cannot be embedded into an
//! array, so we do volatile semantics by hand.
use core::fmt::{self, Write};
use core::ptr::{read_volatile, write_volatile};

use crate::sync::SpinLock;

const VGA_BUFFER: *mut u16 = 0xB8000 as *mut u16;
const WIDTH:  usize = 80;
const HEIGHT: usize = 25;

pub struct Console {
    col:   usize,
    row:   usize,
    color: u8,
}

static CONSOLE: SpinLock<Console> =
    SpinLock::new(Console { col: 0, row: 0, color: 0x07 });

impl Console {
    #[inline(always)]
    fn cell(c: u8, color: u8) -> u16 {
        ((color as u16) << 8) | c as u16
    }

    fn put(&mut self, c: u8) {
        match c {
            b'\n' => { self.row += 1; self.col = 0; }
            b'\r' => { self.col = 0; }
            b'\t' => { self.col = (self.col + 8) & !7; }
            _     => {
                if self.col >= WIDTH { self.row += 1; self.col = 0; }
                if self.row >= HEIGHT { self.scroll(); }
                let idx = self.row * WIDTH + self.col;
                unsafe { write_volatile(VGA_BUFFER.add(idx), Self::cell(c, self.color)); }
                self.col += 1;
            }
        }
        if self.row >= HEIGHT { self.scroll(); }
    }

    fn scroll(&mut self) {
        // Move every row up by one, then blank the last row.
        unsafe {
            for r in 1..HEIGHT {
                for c in 0..WIDTH {
                    let v = read_volatile(VGA_BUFFER.add(r * WIDTH + c));
                    write_volatile(VGA_BUFFER.add((r - 1) * WIDTH + c), v);
                }
            }
            let blank = Self::cell(b' ', self.color);
            for c in 0..WIDTH {
                write_volatile(VGA_BUFFER.add((HEIGHT - 1) * WIDTH + c), blank);
            }
        }
        self.row = HEIGHT - 1;
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &b in s.as_bytes() { self.put(b); }
        Ok(())
    }
}

pub fn write_str(s: &str) {
    let _ = CONSOLE.lock().write_str(s);
}

/// Stub keyboard interrupt handler (called from `arch::interrupts`).
/// Stage 2 will plug `pc-keyboard` here and wake reactor waiters.
pub fn on_keyboard_irq() {
    // intentionally minimal — must not allocate or block.
}
