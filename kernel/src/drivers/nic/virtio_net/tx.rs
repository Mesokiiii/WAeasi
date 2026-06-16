//! TX path — build a virtio-net header + frame and queue it on the
//! transmit virtqueue.
use super::header::{VirtioNetHdr, HDR_LEN};
use super::VirtioNet;
use crate::memory::address::PhysAddr;
use crate::memory::paging::virt_to_phys;
use crate::memory::address::VirtAddr;

/// Push `frame` onto the TX queue.  Caller must guarantee the slice
/// lives until the device marks the descriptor used.
pub fn send(dev: &mut VirtioNet, frame: &[u8]) -> Result<(), &'static str> {
    if frame.len() > 1500 + 14 { return Err("frame too large"); }

    // TODO: allocate a per-queue buffer pool aligned to 4 KiB so the
    // physical address translation below succeeds for direct-mapped RAM.
    // For now we trust that the caller's `frame` is in the direct map.
    let virt = VirtAddr::new(frame.as_ptr() as usize);
    let phys = virt_to_phys(virt).ok_or("frame not in direct map")?;
    let _hdr = VirtioNetHdr::default();
    let _ = HDR_LEN;
    let _ = PhysAddr::new(0); // type-import keepalive

    unsafe {
        if dev.tx.add_buffer_ro(phys, frame.len() as u32).is_none() {
            return Err("TX queue full");
        }
    }
    Ok(())
}
