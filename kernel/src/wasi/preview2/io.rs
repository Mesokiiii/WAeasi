//! `wasi:io/streams` — kernel-side ring buffers.
//!
//! Performance contract:
//!   * `read_into` and `write` are zero-allocation on the hot path —
//!     they copy bytes between the caller's slice and an internal
//!     ring without touching the heap allocator.
//!   * The ring is a fixed-capacity SPSC; producer and consumer race
//!     only on two atomic indices, never on the data area.
//!
//! At trillions of I/O operations / sec, the alloc-free path is the
//! difference between "scales" and "doesn't".
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::wasi::errors::{WasiErr, WasiResult};

/// Default ring capacity — 64 KiB.  Power-of-two so masking replaces
/// modulo on the hot path.
const CAP: usize = 64 * 1024;
const MASK: usize = CAP - 1;

#[repr(C, align(64))]
struct Ring {
    buf:  [u8; CAP],
    head: AtomicUsize,    // producer cursor (writes)
    tail: AtomicUsize,    // consumer cursor (reads)
    closed: AtomicBool,
}

pub struct Stream { ring: Ring }

impl Stream {
    pub fn new() -> alloc::sync::Arc<Self> {
        alloc::sync::Arc::new(Self {
            ring: Ring {
                buf:    [0u8; CAP],
                head:   AtomicUsize::new(0),
                tail:   AtomicUsize::new(0),
                closed: AtomicBool::new(false),
            },
        })
    }

    /// Pull bytes into `dst`.  Returns the byte count actually read.
    /// `Ok(0)` + closed → EOF (mapped by caller to `Pipe`).
    pub fn read_into(&self, dst: &mut [u8]) -> WasiResult<usize> {
        let head = self.ring.head.load(Ordering::Acquire);
        let tail = self.ring.tail.load(Ordering::Relaxed);
        let avail = head.wrapping_sub(tail);
        if avail == 0 {
            return if self.ring.closed.load(Ordering::Acquire) {
                Err(WasiErr::Pipe)
            } else { Ok(0) };
        }
        let take = avail.min(dst.len());
        // Two-segment copy when the slice wraps around the ring.
        let off1 = tail & MASK;
        let seg1 = (CAP - off1).min(take);
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.ring.buf.as_ptr().add(off1), dst.as_mut_ptr(), seg1);
        }
        if take > seg1 {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.ring.buf.as_ptr(),
                    dst.as_mut_ptr().add(seg1),
                    take - seg1);
            }
        }
        self.ring.tail.store(tail.wrapping_add(take), Ordering::Release);
        Ok(take)
    }

    /// Push bytes from `src`; returns the count accepted.  Backpressure
    /// is signalled by a short write — caller awaits a reactor wakeup.
    pub fn write(&self, src: &[u8]) -> WasiResult<usize> {
        if self.ring.closed.load(Ordering::Acquire) { return Err(WasiErr::Pipe); }
        let head = self.ring.head.load(Ordering::Relaxed);
        let tail = self.ring.tail.load(Ordering::Acquire);
        let free = CAP - head.wrapping_sub(tail);
        if free == 0 { return Ok(0); }
        let put = free.min(src.len());
        let off1 = head & MASK;
        let seg1 = (CAP - off1).min(put);
        unsafe {
            core::ptr::copy_nonoverlapping(
                src.as_ptr(), (self.ring.buf.as_ptr() as *mut u8).add(off1), seg1);
        }
        if put > seg1 {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    src.as_ptr().add(seg1),
                    self.ring.buf.as_ptr() as *mut u8,
                    put - seg1);
            }
        }
        self.ring.head.store(head.wrapping_add(put), Ordering::Release);
        Ok(put)
    }

    pub fn close(&self) { self.ring.closed.store(true, Ordering::Release); }

    pub fn pending(&self) -> usize {
        self.ring.head.load(Ordering::Acquire)
            .wrapping_sub(self.ring.tail.load(Ordering::Acquire))
    }
}
