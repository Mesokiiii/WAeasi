//! VFS — root mount table + path resolution.  Used by both `wasi::filesystem`
//! and the kernel itself (e.g. component loader).
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::bitflags;
use spin::Mutex;

use super::inode::Inode;
use super::memfs::MemDir;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u32 {
        const READ   = 1 << 0;
        const WRITE  = 1 << 1;
        const CREATE = 1 << 2;
        const TRUNC  = 1 << 3;
    }
}

#[derive(Debug)]
pub enum VfsError { NotFound, NotDir, AlreadyExists }

struct Mount {
    path: String,
    root: Arc<dyn Inode>,
}

static MOUNTS: Mutex<Vec<Mount>> = Mutex::new(Vec::new());

/// Mount `inode` at `path`. Stage 1 only allows top-level mounts.
pub fn mount(path: &str, root: Arc<dyn Inode>) {
    MOUNTS.lock().push(Mount { path: String::from(path), root });
    log::info!("[vfs] mount '{}' (now {} mounts)", path, MOUNTS.lock().len());
}

/// Hook called once at boot to install the empty in-RAM tree.
pub fn install_default() {
    let root = MemDir::new();
    mount("/", root);
}

/// Returns `(fd, path)` for every preopen — used by `wasi::filesystem::preopens`.
pub fn preopens() -> Vec<(u32, String)> {
    MOUNTS.lock().iter().enumerate()
        .map(|(i, m)| (i as u32, m.path.clone()))
        .collect()
}

/// Resolve `path` and return an opaque inode-fd.  Stage 1 = stub.
pub fn open_at(_parent_fd: u32, _path: &str, _flags: OpenFlags) -> Result<u32, VfsError> {
    Err(VfsError::NotFound)
}
