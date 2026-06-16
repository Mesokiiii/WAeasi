//! VFS-level inode descriptor.  Each filesystem implementation produces its
//! own inodes that conform to this trait.
use alloc::vec::Vec;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileType { Regular, Directory, SymLink, Device }

pub trait Inode: Send + Sync {
    fn file_type(&self) -> FileType;
    fn size(&self)      -> u64;

    fn read (&self, _offset: u64, _buf: &mut [u8]) -> Result<usize, &'static str> {
        Err("read not supported")
    }
    fn write(&self, _offset: u64, _buf: &[u8])     -> Result<usize, &'static str> {
        Err("write not supported")
    }
    fn list (&self) -> Result<Vec<alloc::string::String>, &'static str> {
        Err("not a directory")
    }
}
