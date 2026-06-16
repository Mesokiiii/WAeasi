//! Block-device drivers.  Currently only virtio_blk; NVMe later.
pub mod virtio_blk;

use crate::sched::reactor;

pub fn init() {
    virtio_blk::probe();
}

pub fn on_irq() {
    virtio_blk::on_irq();
    reactor::wake_block_waiters();
}
