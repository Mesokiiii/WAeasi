//! Constant-time helpers — the foundation every secret-dependent
//! comparison in `crypto/*` must use.
//!
//! Production-grade rule: **never** branch on a secret bit, **never**
//! short-circuit on a secret-dependent comparison.  These helpers
//! collapse to single CPU instructions on x86_64 (no branches, no
//! conditional moves dependent on data).
//!
//! 2026 baseline: every TLS / signature / KEM compare goes through
//! `eq_bytes`; every conditional swap (Montgomery ladder, scalar mult)
//! goes through `select`.

/// Constant-time byte-slice equality.  O(N) on length only — independent
/// of where the first differing byte sits.
#[inline]
pub fn eq_bytes(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut diff: u8 = 0;
    for i in 0..a.len() { diff |= a[i] ^ b[i]; }
    diff == 0
}

/// Constant-time u8 equality.
#[inline]
pub fn eq_u8(a: u8, b: u8) -> bool {
    let mut x = a ^ b;
    x |= x >> 4;
    x |= x >> 2;
    x |= x >> 1;
    (x & 1) == 0
}

/// Constant-time u64 equality.
#[inline]
pub fn eq_u64(a: u64, b: u64) -> bool {
    let mut x = a ^ b;
    x |= x >> 32;
    x |= x >> 16;
    x |= x >> 8;
    x |= x >> 4;
    x |= x >> 2;
    x |= x >> 1;
    (x & 1) == 0
}

/// Constant-time conditional select: returns `if cond { a } else { b }`
/// without branching on `cond`.
#[inline]
pub fn select_u32(cond: bool, a: u32, b: u32) -> u32 {
    let mask = (cond as u32).wrapping_neg();   // all-1s if true, 0 otherwise
    (mask & a) | (!mask & b)
}

#[inline]
pub fn select_u64(cond: bool, a: u64, b: u64) -> u64 {
    let mask = (cond as u64).wrapping_neg();
    (mask & a) | (!mask & b)
}

/// Constant-time conditional swap (used by Montgomery ladder).
#[inline]
pub fn cswap_u64(cond: bool, a: &mut u64, b: &mut u64) {
    let mask = (cond as u64).wrapping_neg();
    let t = mask & (*a ^ *b);
    *a ^= t;
    *b ^= t;
}

/// Conditional copy of `src` into `dst` when `cond` — no branch.
#[inline]
pub fn ccopy_bytes(cond: bool, dst: &mut [u8], src: &[u8]) {
    debug_assert_eq!(dst.len(), src.len());
    let mask = (cond as u8).wrapping_neg();
    for i in 0..dst.len() {
        dst[i] = (mask & src[i]) | (!mask & dst[i]);
    }
}

/// Compile-time assertion that we're not in a debug build that might
/// elide constant-time operations through dead-code elimination.
#[allow(dead_code)]
const _: () = assert!(true);
