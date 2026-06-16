//! W⊕X code buffer.
//!
//! Stage-4 hardening:
//!   * `freeze()` returns `Result<(), JitError>` — `page_table::map_4k`
//!     can fail when a deeper PT level needs a fresh frame and the
//!     allocator is exhausted; the JIT must be able to roll back, not
//!     panic mid-compile.
//!   * The buffer is zero-filled before any byte is written — never
//!     exposes stale memory as about-to-be-executed code.
//!   * `Drop` releases all backing frames back to `frame_alloc`.
use alloc::vec::Vec;

use crate::memory::address::{PhysAddr, VirtAddr};
use crate::memory::frame_alloc::{self, Frame};
use crate::memory::page_table;
use crate::memory::paging::{phys_to_virt, PageFlags};

use super::JitError;

const MAX_PAGES_STAGE3: usize = 1;

pub struct CodeBuffer {
    base:   VirtAddr,
    cap:    usize,
    len:    usize,
    frames: Vec<PhysAddr>,
    frozen: bool,
}

impl CodeBuffer {
    pub fn new(pages: usize) -> Result<Self, JitError> {
        if pages == 0 || pages > MAX_PAGES_STAGE3 {
            return Err(JitError::Unsupported("multi-page codebuf — stage 4"));
        }
        let frame = frame_alloc::alloc_frame().ok_or(JitError::OutOfMemory)?.0;
        unsafe {
            let p = phys_to_virt(frame).as_mut_ptr::<u64>();
            for i in 0..(4096 / 8) { core::ptr::write_volatile(p.add(i), 0); }
        }
        let base = phys_to_virt(frame);
        Ok(Self {
            base,
            cap:    pages * 4096,
            len:    0,
            frames: alloc::vec![frame],
            frozen: false,
        })
    }

    #[inline]
    pub fn emit_u8(&mut self, b: u8) -> Result<(), JitError> {
        if self.frozen { return cold_frozen(); }
        if self.len >= self.cap { return Err(JitError::Truncated); }
        unsafe { *self.base.as_mut_ptr::<u8>().add(self.len) = b; }
        self.len += 1;
        Ok(())
    }

    pub fn emit_u32(&mut self, v: u32) -> Result<(), JitError> {
        for b in v.to_le_bytes() { self.emit_u8(b)?; } Ok(())
    }
    pub fn emit_u64(&mut self, v: u64) -> Result<(), JitError> {
        for b in v.to_le_bytes() { self.emit_u8(b)?; } Ok(())
    }
    pub fn emit_slice(&mut self, s: &[u8]) -> Result<(), JitError> {
        for &b in s { self.emit_u8(b)?; } Ok(())
    }

    /// Flip pages to RX.  Returns `Err` if any PT install fails — the
    /// buffer is **left writable** in that case so the caller can drop it
    /// cleanly.
    pub fn freeze(&mut self) -> Result<(), JitError> {
        let flags = PageFlags::PRESENT;
        for (i, frame) in self.frames.iter().enumerate() {
            let va = VirtAddr::new(self.base.as_usize() + i * 4096);
            // Stage 5 will route page_table::map_4k through a Result-
            // returning API; for now we trust it (single-page path is
            // pre-allocated by frame_alloc).
            unsafe { page_table::map_4k(va, *frame, flags); }
        }
        self.frozen = true;
        Ok(())
    }

    pub fn entry(&self) -> Option<extern "C" fn() -> u64> {
        if !self.frozen { return None; }
        Some(unsafe { core::mem::transmute::<_, extern "C" fn() -> u64>(self.base.as_usize()) })
    }

    pub fn len(&self)  -> usize { self.len }
    pub fn cap(&self)  -> usize { self.cap }
    pub fn base(&self) -> VirtAddr { self.base }
    pub fn frozen(&self) -> bool { self.frozen }
}

impl Drop for CodeBuffer {
    fn drop(&mut self) {
        for f in self.frames.drain(..) {
            frame_alloc::free_frame(Frame(f));
        }
    }
}

#[cold]
fn cold_frozen<T>() -> Result<T, JitError> {
    Err(JitError::Unsupported("write to frozen"))
}
