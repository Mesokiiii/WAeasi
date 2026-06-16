//! virtio-net — real driver on top of `drivers::virtio` virtqueues.
//!
//! Two queues:
//!   * **RX (queue 0)** — driver fills with empty buffers; device puts
//!                        packets in.
//!   * **TX (queue 1)** — driver fills with packets to send; device
//!                        marks them used when transmitted.
//!
//! Stage 2 implements one of each at qsz=64.  Multi-queue + checksum
//! offload come in stage 3.
pub mod header;
pub mod rx;
pub mod tx;

use spin::Once;

use crate::sched::reactor;
use crate::drivers::virtio::VirtQueue;
use crate::sync::SpinLock;

pub struct VirtioNet {
    pub mac: [u8; 6],
    pub rx:  VirtQueue,
    pub tx:  VirtQueue,
}

static DEVICE: Once<SpinLock<VirtioNet>> = Once::new();

pub fn install(dev: VirtioNet) {
    DEVICE.call_once(|| SpinLock::new(dev));
}

pub fn probe() {
    // Stage 2: actual probe walks `drivers::pci::scan` and matches
    // (vendor 0x1AF4, device 0x1041 modern).  Until the scan stores
    // results, the probe stays a no-op for the QEMU `-net none` case.
    log::debug!("[virtio_net] probe (modern PCI capability walk)");
}

/// IRQ handler: drain RX into the network stack and wake any pollers.
pub fn on_irq() {
    if let Some(dev) = DEVICE.get() {
        let mut d = dev.lock();
        rx::drain(&mut d);
        reactor::wake_nic_waiters();
    }
}

/// Public TX path used by `wasi::sockets`.
pub fn send_frame(frame: &[u8]) -> Result<(), &'static str> {
    let dev = DEVICE.get().ok_or("virtio_net not present")?;
    let mut d = dev.lock();
    tx::send(&mut d, frame)
}
