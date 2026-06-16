//! Per-instance WASI context.
//!
//! Each Wasm component that requests WASI imports gets a fresh `WasiCtx`
//! holding granted capabilities, file-descriptor table, environment, args.
//!
//! `FdTable` uses a free-list head so `insert` is O(1) instead of O(N).
use alloc::string::String;
use alloc::vec::Vec;

use super::caps::Capability;

pub struct WasiCtx {
    pub caps: Capability,
    pub args: Vec<String>,
    pub env:  Vec<(String, String)>,
    pub fds:  FdTable,
}

impl Default for WasiCtx {
    fn default() -> Self {
        Self {
            caps: Capability::empty(),
            args: Vec::new(),
            env:  Vec::new(),
            fds:  FdTable::new(),
        }
    }
}

pub struct FdTable {
    /// `entries[i]` is either `Slot::Used(entry)` or `Slot::Free(next_free)`.
    entries:    Vec<Slot>,
    free_head:  Option<u32>,
}

enum Slot { Used(FdEntry), Free(Option<u32>) }

pub struct FdEntry {
    pub kind:  FdKind,
    pub flags: u32,
}

pub enum FdKind {
    StdIn,
    StdOut,
    StdErr,
    File(u64),       // VFS inode
    TcpSocket(u32),
    UdpSocket(u32),
}

impl FdTable {
    pub fn new() -> Self {
        let mut t = Self { entries: Vec::with_capacity(8), free_head: None };
        t.insert(FdEntry { kind: FdKind::StdIn,  flags: 0 });
        t.insert(FdEntry { kind: FdKind::StdOut, flags: 0 });
        t.insert(FdEntry { kind: FdKind::StdErr, flags: 0 });
        t
    }

    /// O(1) insert — pop free-list head, fall back to push.
    pub fn insert(&mut self, e: FdEntry) -> u32 {
        if let Some(idx) = self.free_head {
            let next = match &self.entries[idx as usize] {
                Slot::Free(n) => *n,
                Slot::Used(_) => unreachable!("free_head pointed at used slot"),
            };
            self.free_head = next;
            self.entries[idx as usize] = Slot::Used(e);
            idx
        } else {
            self.entries.push(Slot::Used(e));
            (self.entries.len() - 1) as u32
        }
    }

    pub fn get(&self, fd: u32) -> Option<&FdEntry> {
        match self.entries.get(fd as usize) {
            Some(Slot::Used(e)) => Some(e),
            _ => None,
        }
    }

    /// O(1) remove — push freed slot onto the free-list.
    pub fn remove(&mut self, fd: u32) -> Option<FdEntry> {
        let slot = self.entries.get_mut(fd as usize)?;
        let prev = core::mem::replace(slot, Slot::Free(self.free_head));
        match prev {
            Slot::Used(e) => { self.free_head = Some(fd); Some(e) }
            Slot::Free(n) => { *slot = Slot::Free(n); None }
        }
    }

    pub fn len(&self) -> usize { self.entries.len() }
}
