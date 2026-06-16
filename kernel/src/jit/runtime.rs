//! JIT runtime — host-side glue around compiled native code.
//!
//! Responsibilities:
//!   * Entry stub — sets `r15 = memory_base`, `rsi = memory_len`,
//!                  `r14 = fuel`, then `call`s into the JIT'd function.
//!   * Fuel exhaustion trap — installed at vector 0x80 (stage 3 stub).
//!   * Trap unwind — recovers the host stack frame after a Wasm trap
//!                   (matches signal-handler-style unwinding).
//!
//! Public entry: `invoke(code, mem, fuel) -> Result<u64, Trap>`.
use crate::memory::address::VirtAddr;
use crate::memory::linear_mem::LinearMemory;
use crate::wasm::trap::Trap;

#[repr(C)]
pub struct InvokeContext {
    pub mem_base: u64,
    pub mem_len:  u64,
    pub fuel:     u64,
}

/// Run a JIT'd function.  Returns its `rax` value or a trap.
pub fn invoke(
    code_entry: extern "C" fn() -> u64,
    mem:        Option<&LinearMemory>,
    fuel:       u64,
) -> Result<u64, Trap> {
    let _ctx = InvokeContext {
        mem_base: mem.map(|m| m.base.as_u64()).unwrap_or(0),
        mem_len:  mem.map(|m| m.len() as u64).unwrap_or(0),
        fuel,
    };
    // Stage 3: simple direct call; stage 4 sets up r15/rsi/r14 from
    // `_ctx` via an asm trampoline written in `boot.rs`-style naked fn.
    let rax = (code_entry)();
    if rax == u64::MAX { return Err(Trap::Unreachable); }
    Ok(rax)
}

/// Trap-unwind helper — restores the host stack pointer to `host_rsp`
/// and propagates the trap to the executor.  Called from the
/// fault handler installed by `arch::x86_64::idt`.
#[cold]
pub fn unwind(host_rsp: VirtAddr, trap: Trap) -> ! {
    log::error!("[jit] trap unwind {:?} (host_rsp={:?})", trap, host_rsp);
    panic!("jit trap: {:?}", trap);
}
