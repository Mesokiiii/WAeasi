//! Capability tokens — the only way a Wasm component obtains rights to
//! do anything observable.  Modeled after WASI Preview-2's resource model.
use alloc::string::String;
use alloc::vec::Vec;
use bitflags::bitflags;

bitflags! {
    /// Coarse-grained rights granted to a component.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Rights: u64 {
        const FS_READ        = 1 << 0;
        const FS_WRITE       = 1 << 1;
        const NET_CONNECT    = 1 << 2;
        const NET_BIND       = 1 << 3;
        const CLOCK_WALL     = 1 << 4;
        const CLOCK_MONO     = 1 << 5;
        const RANDOM_INSEC   = 1 << 6;
        const RANDOM_SEC     = 1 << 7;
    }
}

#[derive(Debug, Clone)]
pub struct Capability {
    pub rights:  Rights,
    pub tag:     String,    // human-readable label for audit logs
    pub preopen: Vec<String>,
}

impl Capability {
    pub const fn empty() -> Self {
        Self { rights: Rights::empty(), tag: String::new(), preopen: Vec::new() }
    }
}

pub fn init() {
    log::debug!("[wasi::caps] capability subsystem online");
}
