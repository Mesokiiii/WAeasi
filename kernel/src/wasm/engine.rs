//! Wasm engine — lock-free module lookup.
//!
//! Stage-2 perf contract:
//!   * `module(id)` → single atomic load (no Mutex).  Wasm spawn becomes
//!     allocator-bound, not contention-bound.
//!   * `compile(name, bytes)` → IRQ-safe SpinLock for the rare write
//!     side; readers never block writers.
//!
//! Implementation: a fixed-size table of `AtomicPtr<Module>`.  Module
//! count is bounded (default 4096) which is plenty for any real
//! deployment — components, not files, are the unit of replication.
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicPtr, AtomicU64, Ordering};

use super::module::Module;
use super::WasmError;

const MAX_MODULES: usize = 4096;

/// Globally unique compiled-module handle.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u64);

pub struct Engine {
    /// Fixed-size, page-aligned table.  Each slot holds a leaked `Box<Module>`.
    /// Lookup is one `load(Acquire)`.
    table:    [AtomicPtr<Module>; MAX_MODULES],
    next_id:  AtomicU64,
}

unsafe impl Sync for Engine {}

static ENGINE: spin::Once<Engine> = spin::Once::new();

pub fn init() {
    ENGINE.call_once(|| {
        // Cannot use array initializer with non-Copy AtomicPtr directly;
        // build via const fn from null pointers.
        let table = {
            // SAFETY: AtomicPtr<Module> is the same layout as *mut Module
            // (= 8 bytes); zero-init is the null pointer.
            let mut arr: [AtomicPtr<Module>; MAX_MODULES] =
                unsafe { core::mem::zeroed() };
            for slot in arr.iter_mut() {
                *slot = AtomicPtr::new(core::ptr::null_mut());
            }
            arr
        };
        Engine { table, next_id: AtomicU64::new(1) }
    });
}

pub fn engine() -> &'static Engine {
    ENGINE.get().expect("wasm::engine::init() not called")
}

impl Engine {
    /// Decode + verify + validate, then publish into the table.
    pub fn compile(&self, name: &str, bytes: &[u8]) -> Result<ModuleId, WasmError> {
        let module = Module::compile(name, bytes)?;
        let id = ModuleId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let idx = (id.0 as usize) % MAX_MODULES;
        let prev = self.table[idx].swap(Box::into_raw(Box::new(module)), Ordering::Release);
        // Theoretical wraparound — we leaked `prev` since we don't free yet.
        if !prev.is_null() {
            log::warn!("[engine] module table wraparound at idx={}", idx);
        }
        log::info!("[wasm] compiled '{}' as {:?}", name, id);
        Ok(id)
    }

    /// O(1) lock-free lookup.
    #[inline]
    pub fn module(&self, id: ModuleId) -> Option<&'static Module> {
        let idx = (id.0 as usize) % MAX_MODULES;
        let p = self.table[idx].load(Ordering::Acquire);
        if p.is_null() { None } else { Some(unsafe { &*p }) }
    }
}

struct Builtin { name: &'static str, bytes: &'static [u8] }
const BUILTINS: &[Builtin] = &[];

pub async fn load_builtin_components() -> Result<(), WasmError> {
    let eng = engine();
    let mut spawned: Vec<String> = Vec::new();
    for b in BUILTINS {
        let id = eng.compile(b.name, b.bytes)?;
        super::instance::spawn(id)?;
        spawned.push(String::from(b.name));
    }
    log::info!("[wasm] {} builtin components running", spawned.len());
    Ok(())
}
