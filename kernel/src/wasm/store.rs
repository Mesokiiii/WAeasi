//! Per-instance data attached to the Wasm engine: the linear memory
//! handle, the WASI ctx, and any host data the linker wants to expose.
use crate::memory::linear_mem::LinearMemory;
use crate::wasi::ctx::WasiCtx;

pub struct Store {
    pub linear:   LinearMemory,
    pub wasi:     WasiCtx,
    pub fuel:     u64,
    pub epoch:    u64,
}

impl Store {
    pub fn new(linear: LinearMemory, wasi: WasiCtx) -> Self {
        Self { linear, wasi, fuel: 0, epoch: 0 }
    }

    /// Charge `n` units of fuel.  Returns `Err` if the instance ran out.
    pub fn consume_fuel(&mut self, n: u64) -> Result<(), ()> {
        match self.fuel.checked_sub(n) {
            Some(f) => { self.fuel = f; Ok(()) }
            None    => Err(()),
        }
    }

    pub fn add_fuel(&mut self, n: u64) { self.fuel = self.fuel.saturating_add(n); }
}
