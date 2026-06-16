//! Newtype wrappers for physical and virtual addresses.
//!
//! Mixing the two is one of the most common sources of bugs in a kernel —
//! the type system rules it out at compile time.
use core::fmt;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(u64);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(u64);

impl PhysAddr {
    pub const fn new(v: usize) -> Self { Self(v as u64) }
    pub const fn as_u64(self) -> u64   { self.0 }
    pub const fn as_usize(self) -> usize { self.0 as usize }
    pub const fn align_down(self, align: u64) -> Self { Self(self.0 & !(align - 1)) }
    pub const fn align_up(self, align: u64) -> Self {
        Self((self.0 + align - 1) & !(align - 1))
    }
}

impl VirtAddr {
    pub const fn new(v: usize) -> Self { Self(v as u64) }
    pub const fn as_u64(self) -> u64   { self.0 }
    pub const fn as_usize(self) -> usize { self.0 as usize }
    pub const fn as_ptr<T>(self) -> *const T { self.0 as *const T }
    pub const fn as_mut_ptr<T>(self) -> *mut T { self.0 as *mut T }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PA({:#018x})", self.0)
    }
}
impl fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VA({:#018x})", self.0)
    }
}

/// Page size constants.
pub const PAGE_SIZE_4K:  u64 = 4 * 1024;
pub const PAGE_SIZE_2M:  u64 = 2 * 1024 * 1024;
pub const PAGE_SIZE_1G:  u64 = 1024 * 1024 * 1024;
