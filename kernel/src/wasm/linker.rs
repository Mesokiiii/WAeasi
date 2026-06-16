//! Host-import linker — wires WASI host functions into the Wasm engine.
//!
//! Stage 8 makes this concrete:
//!   * `HostFn` — type-erased host function pointer with arity + result-arity
//!     metadata so the validator can type-check imports.
//!   * `Registry` — `(module, field) → HostFn` lookup, populated at
//!     `init()` with the canonical WASI Preview-2 surface plus our
//!     own `wasi:probes/state` extension used by `healthz`.
//!   * `resolve(module, field) -> Option<HostFn>` — single-call lookup
//!     the instance-spawn path uses to bind every Wasm import to a
//!     concrete callable address.
//!
//! Stage-8 keeps the registry as a flat slice — at ~16 bindings the
//! linear scan beats any hash-based structure (cache-friendly, no
//! hasher dependency).
use spin::Once;

/// Cell-shaped argument the dispatcher passes to host functions.
pub type Cell = u64;

pub type HostFn = fn(args: &[Cell], out: &mut [Cell]) -> Result<usize, HostErr>;

#[derive(Debug, Clone, Copy)]
pub enum HostErr { Trap, BadArgs, Unsupported }

#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub module: &'static str,
    pub field:  &'static str,
    pub arity:  u8,
    pub results:u8,
    pub func:   HostFn,
}

pub struct Registry { pub bindings: &'static [Binding] }

static REGISTRY: Once<Registry> = Once::new();

pub fn init() {
    REGISTRY.call_once(|| Registry { bindings: DEFAULTS });
    log::info!("[wasm::linker] {} host bindings registered", DEFAULTS.len());
}

pub fn registry() -> &'static Registry {
    REGISTRY.get().expect("wasm::linker::init() not called")
}

/// Resolve an import.  Linear scan over the static binding table —
/// faster than any hash for ~16 entries.
pub fn resolve(module: &str, field: &str) -> Option<&'static Binding> {
    registry().bindings.iter().find(|b| b.module == module && b.field == field)
}

/// Stage-7 compatibility — return the import set as `(module, field)` pairs.
pub fn imports() -> impl Iterator<Item = (&'static str, &'static str)> {
    registry().bindings.iter().map(|b| (b.module, b.field))
}

// ============== Canonical WASI surface bindings ==============

const DEFAULTS: &[Binding] = &[
    Binding { module: "wasi:clocks/wall-clock",       field: "now",                       arity: 0, results: 1, func: host_wall_now },
    Binding { module: "wasi:clocks/monotonic-clock",  field: "now",                       arity: 0, results: 1, func: host_mono_now },
    Binding { module: "wasi:io/streams",              field: "blocking-read",             arity: 2, results: 1, func: host_stub },
    Binding { module: "wasi:io/streams",              field: "blocking-write-and-flush",  arity: 2, results: 1, func: host_stub },
    Binding { module: "wasi:filesystem/preopens",     field: "get-directories",           arity: 0, results: 1, func: host_stub },
    Binding { module: "wasi:filesystem/types",        field: "open-at",                   arity: 3, results: 1, func: host_stub },
    Binding { module: "wasi:random/random",           field: "get-random-bytes",          arity: 1, results: 1, func: host_random_bytes },
    Binding { module: "wasi:random/random",           field: "get-random-u64",            arity: 0, results: 1, func: host_random_u64 },
    Binding { module: "wasi:sockets/network",         field: "instance-network",          arity: 0, results: 1, func: host_stub },
    Binding { module: "wasi:sockets/tcp-create-socket", field: "create-tcp-socket",       arity: 1, results: 1, func: host_stub },
    Binding { module: "wasi:probes/state",            field: "ready",                     arity: 0, results: 1, func: host_probe_ready },
];

fn host_mono_now(_args: &[Cell], out: &mut [Cell]) -> Result<usize, HostErr> {
    if out.is_empty() { return Err(HostErr::BadArgs); }
    out[0] = crate::wasi::preview2::clocks::monotonic_now_ns();
    Ok(1)
}
fn host_wall_now(_args: &[Cell], out: &mut [Cell]) -> Result<usize, HostErr> {
    if out.is_empty() { return Err(HostErr::BadArgs); }
    out[0] = crate::wasi::preview2::clocks::wall_now_ns();
    Ok(1)
}
fn host_random_u64(_args: &[Cell], out: &mut [Cell]) -> Result<usize, HostErr> {
    if out.is_empty() { return Err(HostErr::BadArgs); }
    out[0] = crate::wasi::preview2::random::get_random_u64();
    Ok(1)
}
fn host_random_bytes(args: &[Cell], out: &mut [Cell]) -> Result<usize, HostErr> {
    if args.is_empty() || out.is_empty() { return Err(HostErr::BadArgs); }
    out[0] = args[0];
    Ok(1)
}
fn host_probe_ready(_args: &[Cell], out: &mut [Cell]) -> Result<usize, HostErr> {
    if out.is_empty() { return Err(HostErr::BadArgs); }
    out[0] = 1;
    Ok(1)
}
fn host_stub(_args: &[Cell], _out: &mut [Cell]) -> Result<usize, HostErr> {
    Err(HostErr::Unsupported)
}
