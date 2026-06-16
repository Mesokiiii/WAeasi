//! `wasi:random/random` — RDRAND-backed CSPRNG with software fallback.
use alloc::vec::Vec;
use core::arch::x86_64::_rdrand64_step;

/// `wasi:random/random.get-random-bytes(len)`.
pub fn get_random_bytes(len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len);
    let mut acc: u64 = 0;
    while out.len() < len {
        if !try_rdrand(&mut acc) {
            // Fallback: xorshift seeded from rdtsc.
            acc ^= crate::arch::x86_64::cpu::rdtsc();
            acc ^= acc.wrapping_shl(13);
            acc ^= acc.wrapping_shr(7);
            acc ^= acc.wrapping_shl(17);
        }
        let bytes = acc.to_le_bytes();
        let need = core::cmp::min(8, len - out.len());
        out.extend_from_slice(&bytes[..need]);
    }
    out
}

/// Insecure 64-bit number — exposed as `wasi:random/random.get-random-u64`.
pub fn get_random_u64() -> u64 {
    let mut v = 0u64;
    if !try_rdrand(&mut v) { v = crate::arch::x86_64::cpu::rdtsc(); }
    v
}

#[inline]
fn try_rdrand(out: &mut u64) -> bool {
    unsafe { _rdrand64_step(out) == 1 }
}
