//! NIC drivers — virtio_net (preferred) and e1000 (legacy fallback).
pub mod e1000;
pub mod virtio_net;

use crate::sched::reactor;

pub fn init() {
    virtio_net::probe();
    if cfg!(feature = "e1000") { e1000::probe(); }
}

/// Called from the NIC IRQ vector — wakes any task awaiting RX.
pub fn on_irq() {
    virtio_net::on_irq();
    reactor::wake_nic_waiters();
}
