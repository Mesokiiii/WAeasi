//! PIT (8254) — used only as a fallback if the LAPIC timer is unavailable.
use crate::arch::x86_64::port::Port;

const PIT_CH0:    Port<u8> = Port::new(0x40);
const PIT_CMD:    Port<u8> = Port::new(0x43);
const PIT_FREQ:   u32       = 1_193_182;

pub fn init() {
    set_frequency(100); // 100 Hz tick — matches `clocks::NS_PER_TICK`
    log::debug!("[timer] PIT @ 100 Hz (fallback)");
}

fn set_frequency(hz: u32) {
    let divisor = (PIT_FREQ / hz) as u16;
    unsafe {
        PIT_CMD.write(0x36);                 // ch0, lo/hi byte, mode 3
        PIT_CH0.write((divisor & 0xFF) as u8);
        PIT_CH0.write((divisor >> 8)  as u8);
    }
}
