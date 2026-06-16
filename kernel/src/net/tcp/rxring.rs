//! Fixed-capacity SPSC ring buffer for TCP RX/TX queues.
//!
//! Replaces `VecDeque<u8>` in `TcpConnection`:
//!
//!   * Bounded — RX growth no longer drives the kernel into OOM.
//!     The window-advertise path now reflects free capacity correctly.
//!   * Zero-allocation `extend` — bytes copy via `copy_nonoverlapping`
//!     into the ring's static-sized backing buffer.
//!   * Cache-friendly — power-of-two capacity → `& MASK` instead of `%`.
//!
//! 64 KiB per connection × 1 M connections = 64 GiB worst-case footprint
//! — same as the legacy Linux per-conn cap.
const CAP:  usize = 64 * 1024;
const MASK: usize = CAP - 1;

#[repr(C, align(64))]
pub struct RxRing {
    buf:  [u8; CAP],
    head: usize,    // producer cursor (writer side)
    tail: usize,    // consumer cursor (reader side)
}

impl core::fmt::Debug for RxRing {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RxRing")
            .field("len",  &self.len())
            .field("free", &self.free())
            .finish()
    }
}

impl RxRing {
    pub fn new() -> Self {
        Self { buf: [0u8; CAP], head: 0, tail: 0 }
    }

    /// Free capacity available for the next push.
    #[inline]
    pub fn free(&self) -> usize { CAP - self.head.wrapping_sub(self.tail) }

    /// Bytes currently buffered.
    #[inline]
    pub fn len(&self) -> usize { self.head.wrapping_sub(self.tail) }

    /// `true` iff push is currently rejected.
    #[inline]
    pub fn is_full(&self) -> bool { self.free() == 0 }

    pub fn is_empty(&self) -> bool { self.head == self.tail }

    /// Append bytes from `src`.  Returns the count actually accepted —
    /// short on backpressure (caller advertises smaller window).
    pub fn push(&mut self, src: &[u8]) -> usize {
        let take = src.len().min(self.free());
        if take == 0 { return 0; }
        let off1 = self.head & MASK;
        let seg1 = (CAP - off1).min(take);
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), self.buf.as_mut_ptr().add(off1), seg1);
            if take > seg1 {
                core::ptr::copy_nonoverlapping(
                    src.as_ptr().add(seg1),
                    self.buf.as_mut_ptr(),
                    take - seg1);
            }
        }
        self.head = self.head.wrapping_add(take);
        take
    }

    /// Drain up to `dst.len()` bytes into `dst`.  Returns count read.
    pub fn pop(&mut self, dst: &mut [u8]) -> usize {
        let avail = self.len();
        if avail == 0 { return 0; }
        let take = avail.min(dst.len());
        let off1 = self.tail & MASK;
        let seg1 = (CAP - off1).min(take);
        unsafe {
            core::ptr::copy_nonoverlapping(self.buf.as_ptr().add(off1), dst.as_mut_ptr(), seg1);
            if take > seg1 {
                core::ptr::copy_nonoverlapping(
                    self.buf.as_ptr(),
                    dst.as_mut_ptr().add(seg1),
                    take - seg1);
            }
        }
        self.tail = self.tail.wrapping_add(take);
        take
    }

    /// Reset both cursors — used after RST/FIN sequence.
    pub fn clear(&mut self) {
        self.head = 0; self.tail = 0;
    }

    /// Advertised receive window in bytes — caps at u16::MAX so it
    /// fits the TCP header field (without window-scale).
    pub fn advertised_window(&self) -> u16 {
        self.free().min(u16::MAX as usize) as u16
    }
}

pub const CAPACITY: usize = CAP;
