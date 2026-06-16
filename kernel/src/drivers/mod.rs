//! Device drivers.  Stage 2 includes virtio + HPET.
pub mod block;
pub mod console;
pub mod hpet;
pub mod nic;
pub mod pci;
pub mod serial;
pub mod timer;
pub mod virtio;

pub fn init() {
    pci::scan();
    timer::init();
    nic::init();
    block::init();
    log::info!("[drivers] initialized");
}
