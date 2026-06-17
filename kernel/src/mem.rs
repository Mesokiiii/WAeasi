//! Scalar `mem*` overrides.
//!
//! Why this exists: our `x86_64-waeasi` target has `-sse,+soft-float`,
//! but the Rust `compiler_builtins` ships `memcpy` / `memset` /
//! `memmove` implementations whose loops the LLVM auto-vectorizer
//! will happily lower to SSE moves anyway.  On a CPU where the OS
//! has not enabled SSE state, those instructions raise `#UD` — and
//! since `core::fmt::write` calls `memcpy` for every formatted
//! string fragment, even our first `log::info!()` blew up.
//!
//! The fix is the canonical kernel-dev workaround: provide our own
//! `extern "C"` implementations with `#[no_mangle]`.  The linker
//! resolves the references inside `compiler_builtins` (and inside
//! every codegen call to `llvm.memcpy.*` / `llvm.memset.*`) to these
//! versions first, so the SSE-bearing fallbacks never get linked.
//!
//! `read_volatile` / `write_volatile` defeat LLVM's loop vectorizer,
//! so these implementations stay as scalar byte loops.

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        core::ptr::write_volatile(dst.add(i), core::ptr::read_volatile(src.add(i)));
        i += 1;
    }
    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let d = dst as usize;
    let s = src as usize;
    if d == s || n == 0 {
        return dst;
    }
    if d < s || d >= s.wrapping_add(n) {
        // Forward copy — non-overlapping or `dst` precedes `src`.
        let mut i = 0;
        while i < n {
            core::ptr::write_volatile(dst.add(i), core::ptr::read_volatile(src.add(i)));
            i += 1;
        }
    } else {
        // Backward copy — `dst` is inside `src..src+n`.
        let mut i = n;
        while i > 0 {
            i -= 1;
            core::ptr::write_volatile(dst.add(i), core::ptr::read_volatile(src.add(i)));
        }
    }
    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dst: *mut u8, val: i32, n: usize) -> *mut u8 {
    let b = val as u8;
    let mut i = 0;
    while i < n {
        core::ptr::write_volatile(dst.add(i), b);
        i += 1;
    }
    dst
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let x = core::ptr::read_volatile(a.add(i));
        let y = core::ptr::read_volatile(b.add(i));
        if x != y {
            return x as i32 - y as i32;
        }
        i += 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn bcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    memcmp(a, b, n)
}
