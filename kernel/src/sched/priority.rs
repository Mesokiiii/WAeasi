//! Priority levels handed to the scheduler.
//!
//! Cloud-native workloads usually fall into a small number of buckets;
//! we expose just enough granularity to let WASI host calls ride above
//! background workers without exposing a real-time API.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Priority {
    /// Background — log shipping, GC, telemetry.
    Idle    = 0,
    /// Default for user Wasm components.
    Normal  = 1,
    /// WASI I/O continuations resumed from an IRQ.
    High    = 2,
    /// Driver / scheduler internals — never blocked by a Wasm trap.
    Kernel  = 3,
}

impl Default for Priority {
    fn default() -> Self { Priority::Normal }
}

impl Priority {
    pub const COUNT: usize = 4;
    pub fn index(self) -> usize { self as usize }
}
