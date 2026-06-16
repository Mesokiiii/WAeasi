//! virtio-blk async I/O glue.
//!
//! Stage 2 ships a synchronous helper (`sync_request`) used by the
//! kernel during boot for filesystem mount.  An async wrapper that
//! parks on `reactor::register_block` ships next.
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use super::{BlockError, VirtioBlk};
use super::request::OpType;
use crate::sched::reactor;

/// Synchronous request — busy-polls the used ring.  Replaced by `Read`/
/// `Write` async futures once we have request-tracking metadata.
pub fn sync_request(
    _dev: &mut VirtioBlk,
    _op: OpType,
    _lba: u64,
    _buf: &mut [u8],
) -> Result<(), BlockError> {
    // Until the descriptor pool + request map are in place this returns
    // `NotPresent`.  Stage-2 deliverable: wire them up — RAM pool of N
    // 512-byte buffers + a `BTreeMap<head_idx, RequestState>`.
    Err(BlockError::NotPresent)
}

/// Drain completed responses; called from the IRQ.
pub fn drain_completions(dev: &mut VirtioBlk) {
    while let Some((head, _len)) = unsafe { dev.queue.take_used() } {
        log::trace!("[virtio_blk] complete head={}", head);
    }
}

/// Async future for `read_block` — registered with the reactor so
/// the executor only re-polls when the IRQ fires.
pub struct BlockRead<'a> {
    pub buf:        &'a mut [u8],
    pub lba:        u64,
    pub registered: bool,
}

impl<'a> Future for BlockRead<'a> {
    type Output = Result<(), BlockError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.registered {
            reactor::register_block(cx.waker().clone());
            self.registered = true;
        }
        // Stage 3 will check a per-request completion flag here.
        Poll::Pending
    }
}
