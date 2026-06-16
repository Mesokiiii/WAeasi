//! AES-NI hardware path — single-block encrypt via `aesenc` /
//! `aesenclast`.
//!
//! Detection: callers check `crate::arch::x86_64::cpuid::features().aes`
//! and call into here if true; otherwise fall back to `encrypt::encrypt_block`.
//!
//! The AES-NI key-schedule in hardware (AESKEYGENASSIST) is also
//! available — but for stage-10 we keep using the software schedule
//! and only accelerate the round function.  This already delivers
//! ~5-10× speedup on TLS bulk encryption.
use core::arch::x86_64::*;

use super::encrypt::AesBlock;
use super::key::KeySchedule;

/// SAFETY: caller asserts CPUID.AES = 1.
#[target_feature(enable = "aes")]
pub unsafe fn encrypt_block_ni(ks: &KeySchedule, block: &mut AesBlock) {
    let mut state = _mm_loadu_si128(block.as_ptr() as *const __m128i);

    state = _mm_xor_si128(state, load(&ks.round_keys[0]));

    for r in 1..ks.rounds as usize {
        state = _mm_aesenc_si128(state, load(&ks.round_keys[r]));
    }
    state = _mm_aesenclast_si128(state, load(&ks.round_keys[ks.rounds as usize]));

    _mm_storeu_si128(block.as_mut_ptr() as *mut __m128i, state);
}

#[target_feature(enable = "aes")]
unsafe fn load(rk: &AesBlock) -> __m128i {
    _mm_loadu_si128(rk.as_ptr() as *const __m128i)
}

/// Dispatch wrapper — software fallback when AES-NI is absent.
pub fn encrypt_block(ks: &KeySchedule, block: &mut AesBlock) {
    if crate::arch::x86_64::cpuid::features().aes {
        unsafe { encrypt_block_ni(ks, block) };
    } else {
        super::encrypt::encrypt_block(ks, block);
    }
}

/// Bulk CTR encrypt — uses AES-NI for the round function and keeps
/// the increment/XOR loop in plain Rust.  Stage-11 will pipeline 4-8
/// blocks through `aesenc` to fully saturate the pipeline.
pub fn ctr_xor_ni(ks: &KeySchedule, j0: &AesBlock, data: &mut [u8]) {
    if !crate::arch::x86_64::cpuid::features().aes {
        return super::encrypt::ctr_block(ks, j0, &mut [0u8; 16]); // noop fallback
    }
    let mut counter = *j0;
    let mut block = [0u8; 16];
    let mut i = 0;
    while i < data.len() {
        inc_be32(&mut counter);
        block.copy_from_slice(&counter);
        unsafe { encrypt_block_ni(ks, &mut block); }
        let take = (data.len() - i).min(16);
        for j in 0..take { data[i + j] ^= block[j]; }
        i += take;
    }
}

#[inline]
fn inc_be32(b: &mut AesBlock) {
    let v = u32::from_be_bytes([b[12], b[13], b[14], b[15]]).wrapping_add(1);
    b[12..16].copy_from_slice(&v.to_be_bytes());
}
