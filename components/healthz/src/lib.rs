//! Readiness / liveness probe component.
#![no_std]

#[cfg(target_arch = "wasm32")]
mod entry {
    extern "C" {
        fn wasi_http_serve(addr: u32, port: u16) -> i32;
        fn wasi_probes_ready() -> u32;
    }

    const ANY_ADDR: u32 = 0;

    #[unsafe(no_mangle)]
    pub extern "C" fn _start() {
        let _ = unsafe { wasi_http_serve(ANY_ADDR, 9100) };
    }

    #[unsafe(no_mangle)]
    pub extern "C" fn handle(path_ptr: *const u8, path_len: usize) -> u32 {
        let path = unsafe { core::slice::from_raw_parts(path_ptr, path_len) };
        match path {
            b"/livez"  => 200,
            b"/readyz" => if unsafe { wasi_probes_ready() } == 1 { 200 } else { 503 },
            _          => 404,
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
