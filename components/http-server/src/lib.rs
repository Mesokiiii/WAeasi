//! HTTP/1.1 server Wasm component.
#![no_std]

#[cfg(target_arch = "wasm32")]
mod entry {
    extern "C" {
        fn wasi_ipc_recv(slot_ptr: *mut u32) -> usize;
        fn wasi_stream_read(fd: u32, ptr: *mut u8, len: usize) -> usize;
        fn wasi_stream_write(fd: u32, ptr: *const u8, len: usize) -> usize;
    }

    const HELLO: &[u8] =
        b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 14\r\n\r\nhello WAeasi!\n";

    #[unsafe(no_mangle)]
    pub extern "C" fn _start() {
        let mut buf = [0u8; 4096];
        loop {
            let mut fd: u32 = 0;
            let got = unsafe { wasi_ipc_recv(&mut fd as *mut u32) };
            if got == 0 { return; }

            let _ = unsafe { wasi_stream_read(fd, buf.as_mut_ptr(), buf.len()) };
            let _ = unsafe { wasi_stream_write(fd, HELLO.as_ptr(), HELLO.len()) };
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
