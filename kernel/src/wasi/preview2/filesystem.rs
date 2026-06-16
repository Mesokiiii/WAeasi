//! `wasi:filesystem/*` — bridges to the kernel VFS.
use alloc::string::String;
use alloc::vec::Vec;

use crate::fs::vfs::{self, OpenFlags};
use crate::wasi::errors::{WasiErr, WasiResult};

/// `wasi:filesystem/preopens.get-directories` — list all preopened dirs.
pub fn get_preopens() -> Vec<(u32, String)> {
    vfs::preopens()
}

/// `wasi:filesystem/types.open-at` — open a path relative to a directory fd.
pub fn open_at(parent_fd: u32, path: &str, flags: OpenFlags) -> WasiResult<u32> {
    vfs::open_at(parent_fd, path, flags).map_err(|_| WasiErr::Io)
}

/// `wasi:filesystem/types.read-via-stream` — return a stream id for `fd`.
pub fn read_via_stream(_fd: u32, _offset: u64) -> WasiResult<u32> {
    // TODO: wire to fs::vfs::read_stream
    Err(WasiErr::NotSup)
}

/// `wasi:filesystem/types.write-via-stream`.
pub fn write_via_stream(_fd: u32, _offset: u64) -> WasiResult<u32> {
    Err(WasiErr::NotSup)
}
