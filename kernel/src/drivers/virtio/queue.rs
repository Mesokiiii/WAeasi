//! High-level VirtQueue.
//!
//! Hot-path tightness:
//!   * `add_buffer_*`  — exactly one descriptor write + one ring slot
//!                       write + one `Release` fence + one idx bump.
//!   * `take_used`     — minimal recycle: only the descriptor's `next`
//!                       field is written (atomic 16-bit store), so the
//!                       device can never observe a half-recycled
//!                       descriptor's other fields.
use core::ptr::{read_volatile, write_volatile};

use super::descriptor::Descriptor;
use super::ring::{layout_for, Layout};
use crate::memory::address::{PhysAddr, VirtAddr};

pub struct VirtQueue {
    pub qsz:       u16,
    pub layout:    Layout,
    pub base_phys: PhysAddr,
    pub base_virt: VirtAddr,
    free_head:  u16,
    free_count: u16,
    last_used:  u16,
}

impl VirtQueue {
    pub unsafe fn init(qsz: u16, base_phys: PhysAddr, base_virt: VirtAddr) -> Self {
        let layout = layout_for(qsz as usize);
        let descs_ptr = base_virt.as_mut_ptr::<Descriptor>();
        for i in 0..qsz {
            let next = if i + 1 == qsz { 0 } else { i + 1 };
            write_volatile(descs_ptr.add(i as usize), Descriptor {
                addr: 0, len: 0, flags: 0, next,
            });
        }
        Self { qsz, layout, base_phys, base_virt,
               free_head: 0, free_count: qsz, last_used: 0 }
    }

    #[inline]
    pub unsafe fn add_buffer_ro(&mut self, addr: PhysAddr, len: u32) -> Option<u16> {
        self.add_buffer(addr, len, false)
    }
    #[inline]
    pub unsafe fn add_buffer_wo(&mut self, addr: PhysAddr, len: u32) -> Option<u16> {
        self.add_buffer(addr, len, true)
    }

    #[inline]
    unsafe fn add_buffer(&mut self, addr: PhysAddr, len: u32, write: bool) -> Option<u16> {
        if self.free_count == 0 { return None; }
        let head = self.free_head;
        let descs = self.base_virt.as_mut_ptr::<Descriptor>();
        let next = read_volatile(descs.add(head as usize)).next;

        let desc = if write {
            Descriptor::write_only(addr.as_u64(), len, None)
        } else {
            Descriptor::read_only(addr.as_u64(), len, None)
        };
        write_volatile(descs.add(head as usize), desc);

        self.free_head  = next;
        self.free_count -= 1;
        self.publish(head);
        Some(head)
    }

    unsafe fn publish(&mut self, head: u16) {
        let avail_ptr = (self.base_virt.as_usize() + self.layout.avail_off) as *mut u16;
        let idx_ptr = avail_ptr.add(1);
        let cur_idx = read_volatile(idx_ptr);
        let ring_slot = avail_ptr.add(2 + (cur_idx as usize % self.qsz as usize));
        write_volatile(ring_slot, head);
        // Ensure the ring write retires before the device polls idx.
        core::sync::atomic::fence(core::sync::atomic::Ordering::Release);
        write_volatile(idx_ptr, cur_idx.wrapping_add(1));
    }

    /// Consume next completed descriptor.  Returns `(head, bytes_written)`.
    /// Recycling writes only the `next` field — the device never sees a
    /// torn descriptor.
    pub unsafe fn take_used(&mut self) -> Option<(u16, u32)> {
        let used_ptr = (self.base_virt.as_usize() + self.layout.used_off) as *mut u16;
        let idx_ptr = used_ptr.add(1);
        let used_idx = read_volatile(idx_ptr);
        if used_idx == self.last_used { return None; }
        let elem_ptr = (used_ptr.add(2)) as *mut super::ring::UsedElem;
        let elem = read_volatile(elem_ptr.add(self.last_used as usize % self.qsz as usize));
        self.last_used = self.last_used.wrapping_add(1);

        // Recycle: write only the `next` field (16-bit, atomic).
        let descs = self.base_virt.as_mut_ptr::<Descriptor>();
        let next_field_ptr = (descs.add(elem.id as usize) as *mut u8)
            .add(core::mem::offset_of!(Descriptor, next))
            as *mut u16;
        write_volatile(next_field_ptr, self.free_head);
        self.free_head  = elem.id as u16;
        self.free_count += 1;

        Some((elem.id as u16, elem.len))
    }

    #[inline] pub fn free_slots(&self) -> u16 { self.free_count }
}
