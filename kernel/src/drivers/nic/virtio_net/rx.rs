//! RX path — drain completed buffers into the network stack.
use super::header::HDR_LEN;
use super::VirtioNet;
use crate::net::ethernet;

/// Drain every used buffer from `dev.rx` and feed each frame into the
/// kernel's net stack.  Called from the NIC IRQ.
pub fn drain(dev: &mut VirtioNet) {
    while let Some((head, len)) = unsafe { dev.rx.take_used() } {
        if (len as usize) <= HDR_LEN { continue; }
        // `head` is the descriptor index; the buffer's virtual address
        // lives in the descriptor table.  Stage 3 will track buffer
        // pointers explicitly so we don't re-walk on each completion.
        let _ = (head, len);
        log::trace!("[virtio_net] RX {} bytes", len);
        // TODO: feed frame slice into `ethernet::parse` once the buffer
        // table is wired up.
        let _ = ethernet::parse;
    }
}
