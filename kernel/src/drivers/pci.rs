//! Minimal PCI bus enumeration over the legacy CF8h/CFCh config space.
use crate::arch::x86_64::port::Port;

const CONFIG_ADDR: Port<u32> = Port::new(0xCF8);
const CONFIG_DATA: Port<u32> = Port::new(0xCFC);

#[derive(Copy, Clone, Debug)]
pub struct PciDevice {
    pub bus: u8,
    pub dev: u8,
    pub fun: u8,
    pub vendor: u16,
    pub device: u16,
    pub class:  u8,
    pub subclass: u8,
}

fn read_word(bus: u8, dev: u8, fun: u8, offset: u8) -> u32 {
    let addr = 0x8000_0000
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((fun as u32) << 8)
        | ((offset as u32) & 0xFC);
    unsafe {
        CONFIG_ADDR.write(addr);
        CONFIG_DATA.read()
    }
}

/// Enumerate every present PCI function, log each one.
pub fn scan() {
    let mut count = 0usize;
    for bus in 0..=255u8 {
        for dev in 0..32u8 {
            for fun in 0..8u8 {
                let id = read_word(bus, dev, fun, 0);
                let vendor = (id & 0xFFFF) as u16;
                if vendor == 0xFFFF { continue; }
                let device = (id >> 16) as u16;
                let class_word = read_word(bus, dev, fun, 8);
                let class    = (class_word >> 24) as u8;
                let subclass = (class_word >> 16) as u8;
                log::info!("[pci] {:02x}:{:02x}.{} {:04x}:{:04x} class={:02x}:{:02x}",
                           bus, dev, fun, vendor, device, class, subclass);
                count += 1;
            }
        }
    }
    log::info!("[pci] {} devices found", count);
}
