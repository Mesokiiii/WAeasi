//! WASI — the **only** ABI exposed to user code in WAeasi.
//!
//! There is no POSIX, no ioctl, no syscall table.  Components import
//! capabilities, the kernel decides which ones to grant.  Isolation is
//! provided by the Wasm sandbox; the kernel only checks capability tokens.
pub mod caps;
pub mod ctx;
pub mod errors;
pub mod preview2;

pub fn init() {
    caps::init();
    log::info!("[wasi] preview-2 host functions registered");
}
