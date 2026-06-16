//! WAeasi — bare-metal WebAssembly Microkernel.
#![no_std]
#![cfg_attr(not(test), no_main)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(static_mut_refs)]
#![allow(clippy::missing_safety_doc)]
// Pedantic style guides we intentionally don't follow — these are
// established kernel-codebase conventions (see Linux, Redox, hubris):
#![allow(clippy::uninlined_format_args)]   // `format!("{}", x)` is fine.
#![allow(clippy::doc_lazy_continuation)]   // doc-list indent is cosmetic.
#![allow(clippy::new_without_default)]     // explicit `new` is preferred.
#![allow(clippy::result_unit_err)]         // `Result<_, ()>` is OK for booleans-with-context.
#![allow(clippy::needless_lifetimes)]      // explicit lifetimes aid readability.
#![allow(clippy::needless_range_loop)]     // index-based loops are clearer in low-level code.
#![allow(clippy::manual_div_ceil)]         // explicit `(a+b-1)/b` is clearer in crypto.

extern crate alloc;

pub mod acpi;
pub mod admin;
pub mod allocator;
pub mod arch;
pub mod boot;
pub mod crypto;
pub mod debug;
pub mod drivers;
pub mod fs;
pub mod http;
pub mod ipc;
pub mod jit;
pub mod log_;
pub mod memory;
pub mod net;
pub mod obs;
pub mod panic_;
pub mod sched;
pub mod security;
pub mod sync;
pub mod wasi;
pub mod wasm;

pub const BANNER: &str = "\
======================================================================\n\
 WAeasi — Bare-metal WebAssembly Microkernel  (Cloud-Native OS)        \n\
   * single address space   * async-first   * WASI-only ABI            \n\
   * Stage 10 (final): ML-KEM math + ML-DSA + AES-NI + Argon2id +     \n\
                       real TLS handshake + admin TCP listener        \n\
======================================================================\n";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn kernel_main() -> ! {
    log::info!("{}", BANNER);
    log::info!("kernel v{} booting...", VERSION);
    sched::executor::Executor::global().run()
}
