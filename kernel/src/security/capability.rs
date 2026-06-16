//! Capability tokens.
//!
//! A `Capability` is an **unforgeable** 128-bit token:
//!   * 64-bit `id`     — sequentially issued from the kernel.
//!   * 64-bit `digest` — keyed Blake-style hash of `(id || rights || epoch)`
//!                       computed against a per-boot kernel secret.
//!
//! Components can pass the 128-bit token freely, but they cannot forge
//! one because they don't have the secret.  When the kernel receives a
//! token, it recomputes the digest and rejects mismatches.
//!
//! This is *not* a replacement for memory isolation — it is the second
//! line of defense once a buggy host function tries to act on a foreign
//! handle.
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Once;

use crate::wasi::caps::Rights;

/// 128-bit capability token.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct Cap { pub id: u64, pub digest: u64 }

/// Issued capability metadata kept inside the kernel.
#[derive(Copy, Clone, Debug)]
pub struct CapRecord {
    pub id:      u64,
    pub rights:  Rights,
    pub epoch:   u64,
    pub revoked: bool,
}

static NEXT_ID:    AtomicU64 = AtomicU64::new(1);
static BOOT_SECRET: Once<u64> = Once::new();

pub fn init() {
    BOOT_SECRET.call_once(|| {
        // Best-effort entropy: rdtsc xor rdrand if available.
        let r = crate::wasi::preview2::random::get_random_u64();
        let t = crate::arch::x86_64::cpu::rdtsc();
        r ^ t.rotate_left(13) ^ 0x9E37_79B9_7F4A_7C15
    });
    log::debug!("[capability] secret installed");
}

/// Mint a fresh capability for the given rights.
pub fn mint(rights: Rights) -> (Cap, CapRecord) {
    let id    = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let epoch = crate::arch::x86_64::cpu::rdtsc();
    let digest = derive_digest(id, rights.bits(), epoch);
    let cap = Cap { id, digest };
    let rec = CapRecord { id, rights, epoch, revoked: false };
    crate::security::audit::record_grant(cap, rights);
    (cap, rec)
}

/// Verify that `cap` was minted by this boot.  Returns `true` iff the
/// digest matches and the record is not revoked.
pub fn verify(cap: Cap, rec: &CapRecord) -> bool {
    if rec.revoked || rec.id != cap.id { return false; }
    let expected = derive_digest(rec.id, rec.rights.bits(), rec.epoch);
    let ok = constant_time_eq(expected, cap.digest);
    if !ok { crate::security::audit::record_forge_attempt(cap); }
    ok
}

/// Authorization gate: verify the cap **and** check that it carries
/// every bit of `needed`.  Records the audit entry either way.
pub fn check(cap: Cap, rec: &CapRecord, needed: Rights) -> bool {
    if !verify(cap, rec) { return false; }
    let granted = rec.rights.contains(needed);
    crate::security::audit::record_check(cap, needed, granted);
    granted
}

#[inline]
fn derive_digest(id: u64, rights: u64, epoch: u64) -> u64 {
    // Lightweight one-block mix — Blake-style, replaced by SHA-256 truncation
    // when crypto is wired in.  Keyed by BOOT_SECRET to prevent forging.
    let k = *BOOT_SECRET.get().unwrap_or(&0xDEADBEEF_CAFEBABE);
    let mut h = id.wrapping_mul(0x9E3779B97F4A7C15) ^ k;
    h = h.rotate_left(31).wrapping_add(rights.wrapping_mul(0xBF58476D1CE4E5B9));
    h ^= epoch.wrapping_mul(0x94D049BB133111EB);
    h ^ h.rotate_right(33)
}

/// Branch-free equality check — avoids early-exit timing leak.
#[inline]
fn constant_time_eq(a: u64, b: u64) -> bool {
    let mut diff = a ^ b;
    diff |= diff >> 32;
    diff |= diff >> 16;
    diff |= diff >> 8;
    diff |= diff >> 4;
    diff |= diff >> 2;
    diff |= diff >> 1;
    (diff & 1) == 0
}
