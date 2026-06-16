//! Bitmap-backed buddy tree.
//!
//! For an arena of `2^max_order` bytes the tree has `2^(max_order-min_order+1) - 1`
//! nodes; each node stores the *largest free order* in its subtree.  That
//! gives O(log N) alloc/free.
//!
//! The tree itself lives inside the arena (self-hosted) — we reserve a
//! prefix of the arena big enough to hold the bitmap.
use alloc::vec::Vec;

pub struct BuddyTree {
    base:        usize,
    arena_size:  usize,
    min_order:   u8,
    max_order:   u8,
    /// `nodes[i]` = largest order still allocatable inside subtree `i`.
    /// `0` means fully allocated.  Indexed in heap-array form (1-based).
    nodes:       Vec<u8>,
    free_bytes:  usize,
}

impl BuddyTree {
    pub const fn empty() -> Self {
        Self {
            base: 0, arena_size: 0, min_order: 0, max_order: 0,
            nodes: Vec::new(), free_bytes: 0,
        }
    }

    pub fn init(&mut self, base: usize, len: usize, min: u8, max: u8) {
        let span = round_up_pow2(len);
        let levels = (max - min + 1) as usize;
        let n_nodes = (1usize << levels) - 1;
        self.base = base;
        self.arena_size = span;
        self.min_order = min;
        self.max_order = max;
        self.nodes = alloc::vec![max; n_nodes + 1]; // 1-based heap index
        self.nodes[0] = 0; // unused
        self.free_bytes = span;
    }

    /// Allocate a block of `2^order` bytes.  Returns the absolute address.
    pub fn alloc(&mut self, order: u8) -> Option<usize> {
        if order < self.min_order || order > self.max_order { return None; }
        if self.nodes[1] < order { return None; }

        let mut idx = 1;
        let mut cur_order = self.max_order;
        while cur_order > order {
            let l = idx * 2;
            let r = idx * 2 + 1;
            // Pick a child capable of fitting `order`.
            if self.nodes[l] >= order      { idx = l; }
            else if self.nodes[r] >= order { idx = r; }
            else { return None; } // shouldn't happen given root check
            cur_order -= 1;
        }

        // Mark fully allocated.
        self.nodes[idx] = 0;
        let block_size = 1usize << order;
        let offset = ((idx + 1) << order) - self.arena_size;
        let abs = self.base + offset;
        self.free_bytes -= block_size;
        // Propagate "largest-free" upward.
        self.update_up(idx);
        Some(abs)
    }

    pub fn free(&mut self, addr: usize, order: u8) {
        if order < self.min_order || order > self.max_order { return; }
        let offset = addr - self.base;
        let idx = ((offset + self.arena_size) >> order).max(1);
        self.nodes[idx] = order;
        self.free_bytes += 1usize << order;

        // Coalesce upward where possible.
        let mut i = idx;
        let mut cur = order;
        while i > 1 && cur < self.max_order {
            let parent = i / 2;
            let buddy  = i ^ 1;
            if self.nodes[buddy] == cur {
                self.nodes[parent] = cur + 1;
                self.nodes[i] = 0;
                self.nodes[buddy] = 0;
                i = parent;
                cur += 1;
            } else {
                break;
            }
        }
        self.update_up(idx);
    }

    pub fn free_bytes(&self) -> usize { self.free_bytes }

    fn update_up(&mut self, mut idx: usize) {
        while idx > 1 {
            idx /= 2;
            let l = self.nodes[idx * 2];
            let r = self.nodes[idx * 2 + 1];
            self.nodes[idx] = l.max(r);
        }
    }
}

fn round_up_pow2(mut x: usize) -> usize {
    if x == 0 { return 1; }
    x -= 1;
    x |= x >> 1; x |= x >> 2;  x |= x >> 4;
    x |= x >> 8; x |= x >> 16; x |= x >> 32;
    x + 1
}
