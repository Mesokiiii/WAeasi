//! TLS-terminator Wasm component.
#![no_std]

#[cfg(target_arch = "wasm32")]
mod entry {
    extern "C" {
        fn wasi_tcp_accept(listener_fd: u32) -> u32;
        fn wasi_tls_serve(socket_fd: u32, plaintext_pipe: u32) -> u32;
        fn wasi_pipe_create() -> u64;
        fn wasi_ipc_send(target: u32, data_ptr: *const u8, data_len: usize);
    }

    const HTTP_BACKEND: u32 = 0xC0FFEE;
    const TLS_LISTENER: u32 = 0;

    #[unsafe(no_mangle)]
    pub extern "C" fn _start() {
        loop {
            let socket = unsafe { wasi_tcp_accept(TLS_LISTENER) };
            let pipe   = unsafe { wasi_pipe_create() };
            let read_fd  = (pipe >> 32) as u32;
            let write_fd = pipe as u32;

            let _ = unsafe { wasi_tls_serve(socket, write_fd) };
            unsafe {
                wasi_ipc_send(HTTP_BACKEND,
                              &read_fd as *const u32 as *const u8,
                              core::mem::size_of::<u32>());
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
