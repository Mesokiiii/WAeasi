//! Virtual filesystem.  WASI components see only what their capabilities
//! preopen, so the VFS exposes a clean tree without device files.
pub mod inode;
pub mod memfs;
pub mod vfs;
