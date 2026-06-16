//! Simplest possible WAeasi component — writes a banner to stdout via WASI.
//!
//! Built for `wasm32-wasi-preview2`.  The kernel loads this `.wasm` file
//! through `wasm::engine::compile` + `wasm::instance::spawn`.
#![no_std]

#[cfg(target_arch = "wasm32")]
mod wasm_entry {
    extern "C" {
        fn wasi_write(fd: u32, ptr: *const u8, len: usize) -> u32;
    }

    const MSG: &str = "hello from WAeasi component!\n";

    #[unsafe(no_mangle)]
    pub extern "C" fn _start() {
        unsafe {
            let _ = wasi_write(1, MSG.as_ptr(), MSG.len());
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
