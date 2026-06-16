//! virtio-blk — async block-device driver on top of the virtqueue.
//!
//! API:
//!   * `read_block(lba, &mut buf) -> Future<Output=Result<()>>`
//!   * `write_block(lba, &buf)    -> Future<Output=Result<()>>`
//!
//! Stage 2 ships the request/header layouts + sync stubs; the async
//! path is wired through `reactor::register_block`.
pub mod async_io;
pub mod request;

use spin::Once;

use crate::drivers::virtio::VirtQueue;
use crate::sched::reactor;
use crate::sync::SpinLock;

#[derive(Debug)]
pub enum BlockError { NotPresent, Io, OutOfBounds }

pub struct VirtioBlk {
    pub capacity_bytes: u64,
    pub block_size:     u32,
    pub queue:          VirtQueue,
}

static DEVICE: Once<SpinLock<VirtioBlk>> = Once::new();

pub fn install(dev: VirtioBlk) {
    DEVICE.call_once(|| SpinLock::new(dev));
}

pub fn probe() {
    log::debug!("[virtio_blk] probe (modern PCI capability walk)");
}

pub fn on_irq() {
    if let Some(dev) = DEVICE.get() {
        let mut d = dev.lock();
        async_io::drain_completions(&mut d);
        reactor::wake_block_waiters();
    }
}

pub fn read_block(lba: u64, buf: &mut [u8]) -> Result<(), BlockError> {
    let dev = DEVICE.get().ok_or(BlockError::NotPresent)?;
    let mut d = dev.lock();
    async_io::sync_request(&mut d, request::OpType::Read, lba, buf)
}

pub fn write_block(lba: u64, buf: &[u8]) -> Result<(), BlockError> {
    let dev = DEVICE.get().ok_or(BlockError::NotPresent)?;
    let mut d = dev.lock();
    // SAFETY: write path doesn't mutate `buf` — but our async helper
    // takes &mut.  Cast away the const for the helper, never write.
    let buf_mut = unsafe { core::slice::from_raw_parts_mut(buf.as_ptr() as *mut u8, buf.len()) };
    async_io::sync_request(&mut d, request::OpType::Write, lba, buf_mut)
}
