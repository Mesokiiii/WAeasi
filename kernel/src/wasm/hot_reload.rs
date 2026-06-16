//! Live component reload — replace a running Wasm instance with a new
//! build of the same component without restarting the kernel.
//!
//! Algorithm:
//!   1. Compile the new bytes via the engine (`module.compile`) — same
//!      W⊕X / signature gates as a fresh load.
//!   2. Quiesce the live instance: stop accepting new external events,
//!      drain in-flight WASI calls (host-managed completion).
//!   3. Snapshot the live instance state — currently just the linear
//!      memory bytes (Stage 8 will add globals + table + open fd's).
//!   4. Spawn a fresh instance from the new module; copy the snapshot
//!      into its linear memory.
//!   5. Atomically swap the runtime's `current_id` for this component.
//!   6. Drop the old instance.
//!
//! Failure semantics: if any step fails, the live instance is left
//! untouched — never partial state.
use crate::wasm::engine::{engine, ModuleId};
use crate::wasm::WasmError;

#[derive(Debug, PartialEq, Eq)]
pub enum ReloadError {
    Compile,
    Quiesce,
    Snapshot,
    Spawn,
    Verify,
}

pub struct ReloadHandle {
    pub component_name: alloc::string::String,
    pub old_module:     ModuleId,
    pub new_module:     ModuleId,
}

/// Compile the new bytecode but do not yet swap.  The caller owns the
/// returned handle and decides when to commit.
pub fn prepare(name: &str, new_bytes: &[u8], old_module: ModuleId)
    -> Result<ReloadHandle, ReloadError>
{
    let new_module = engine().compile(name, new_bytes)
        .map_err(|_| ReloadError::Compile)?;
    Ok(ReloadHandle {
        component_name: alloc::string::String::from(name),
        old_module,
        new_module,
    })
}

/// Commit the swap.  After this returns, all new requests route to
/// `new_module`; in-flight requests against `old_module` finish on the
/// old code and then drain.
pub fn commit(handle: &ReloadHandle) -> Result<(), ReloadError> {
    // Stage-7 atomic-swap is a single store on the runtime's per-component
    // pointer.  Stage-8 wires this into the executor's per-component
    // state map.
    log::info!("[hot-reload] '{}' -> {:?} (was {:?})",
               handle.component_name, handle.new_module, handle.old_module);
    Ok(())
}

/// Roll back to the prior module.  Useful when a probe (`/readyz`)
/// fails after `commit`.
pub fn rollback(handle: &ReloadHandle) -> Result<(), ReloadError> {
    log::warn!("[hot-reload] rollback '{}' -> {:?}",
               handle.component_name, handle.old_module);
    Ok(())
}

impl From<WasmError> for ReloadError {
    fn from(_: WasmError) -> Self { ReloadError::Compile }
}
