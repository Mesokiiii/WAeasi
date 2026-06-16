//! Slab caches — fixed-size, lock-free fast path.
//!
//! Each cache is a Treiber stack of free objects of `size_class` bytes.
//! Refilling pops a single buddy block and slabs it; freeing pushes the
//! object directly back without touching the buddy.  This gives the
//! kernel O(1) allocation for the common case (Wasm Task/Frame structs).
use core::ptr::null_mut;
use core::sync::atomic::{AtomicPtr, Ordering};

#[repr(C)]
struct SlabNode { next: AtomicPtr<SlabNode> }

pub struct Slab {
    size:   usize,
    align:  usize,
    head:   AtomicPtr<SlabNode>,
}

impl Slab {
    pub const fn new(size: usize, align: usize) -> Self {
        Self { size, align, head: AtomicPtr::new(null_mut()) }
    }

    /// O(1) pop.  Returns `None` on empty slab — caller must refill.
    pub fn alloc(&self) -> Option<*mut u8> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            if head.is_null() { return None; }
            let next = unsafe { (*head).next.load(Ordering::Acquire) };
            if self.head
                .compare_exchange_weak(head, next, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Some(head as *mut u8);
            }
        }
    }

    /// O(1) push.
    pub unsafe fn free(&self, p: *mut u8) {
        let node = p as *mut SlabNode;
        let mut cur = self.head.load(Ordering::Acquire);
        loop {
            (*node).next.store(cur, Ordering::Relaxed);
            match self.head
                .compare_exchange_weak(cur, node, Ordering::Release, Ordering::Acquire)
            {
                Ok(_) => return,
                Err(actual) => cur = actual,
            }
        }
    }

    /// Refill from a contiguous block of `len` bytes — slice it into
    /// `len / size` objects and prepend them to the free list.
    pub unsafe fn refill(&self, block: *mut u8, len: usize) {
        let count = len / self.size;
        for i in 0..count {
            let p = block.add(i * self.size);
            self.free(p);
        }
    }

    pub fn size(&self) -> usize { self.size }
    pub fn align(&self) -> usize { self.align }
}
