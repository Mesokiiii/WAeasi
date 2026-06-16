//! In-RAM filesystem — used for the read-only `/components` tree shipped
//! inside the kernel image, and for ephemeral scratch.
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use super::inode::{FileType, Inode};

pub struct MemFile {
    bytes: Mutex<Vec<u8>>,
}

impl MemFile {
    pub fn new(initial: Vec<u8>) -> Arc<Self> {
        Arc::new(Self { bytes: Mutex::new(initial) })
    }
}

impl Inode for MemFile {
    fn file_type(&self) -> FileType { FileType::Regular }
    fn size(&self) -> u64 { self.bytes.lock().len() as u64 }
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<usize, &'static str> {
        let bytes = self.bytes.lock();
        let off = offset as usize;
        if off >= bytes.len() { return Ok(0); }
        let n = core::cmp::min(buf.len(), bytes.len() - off);
        buf[..n].copy_from_slice(&bytes[off..off + n]);
        Ok(n)
    }
    fn write(&self, offset: u64, buf: &[u8]) -> Result<usize, &'static str> {
        let mut bytes = self.bytes.lock();
        let off = offset as usize;
        if bytes.len() < off + buf.len() { bytes.resize(off + buf.len(), 0); }
        bytes[off..off + buf.len()].copy_from_slice(buf);
        Ok(buf.len())
    }
}

pub struct MemDir {
    children: Mutex<BTreeMap<String, Arc<dyn Inode>>>,
}

impl MemDir {
    pub fn new() -> Arc<Self> {
        Arc::new(Self { children: Mutex::new(BTreeMap::new()) })
    }
    pub fn insert(&self, name: &str, node: Arc<dyn Inode>) {
        self.children.lock().insert(String::from(name), node);
    }
    pub fn lookup(&self, name: &str) -> Option<Arc<dyn Inode>> {
        self.children.lock().get(name).cloned()
    }
}

impl Inode for MemDir {
    fn file_type(&self) -> FileType { FileType::Directory }
    fn size(&self) -> u64 { self.children.lock().len() as u64 }
    fn list(&self) -> Result<Vec<String>, &'static str> {
        Ok(self.children.lock().keys().cloned().collect())
    }
}
