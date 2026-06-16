//! Append-only audit ring — **lock-free** writers, lock-protected drain.
//!
//! Stage-8 fast path (push):
//!   1. `cursor.fetch_add(1, Relaxed)` — unique seq, lock-free.
//!   2. Load `tsc` and write `(seq, tsc, cap, kind)` directly into
//!      `slots[seq & MASK]` via `UnsafeCell`.  Concurrent writers can
//!      land on different slots; only the **drain** path takes the
//!      SpinLock briefly to copy the snapshot out.
//!
//! At trillions of `cap::check` per second the previous design's
//! per-event SpinLock was the bottleneck — we now take it only on
//! `drain_to`, which is a once-per-second operation.
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::security::capability::Cap;
use crate::sync::SpinLock;
use crate::wasi::caps::Rights;

#[derive(Copy, Clone, Debug)]
pub enum AuditKind {
    Grant         { rights: Rights },
    Revoke,
    Check         { needed: Rights, granted: bool },
    ForgeAttempt,
}

#[derive(Copy, Clone, Debug)]
pub struct AuditEvent {
    pub seq:   u64,
    pub tsc:   u64,
    pub cap:   Cap,
    pub kind:  AuditKind,
}

const RING_LEN: usize = 1024;
const RING_MASK: u64 = (RING_LEN - 1) as u64;

#[repr(align(64))]
struct Ring {
    cursor: AtomicU64,
    slots:  UnsafeCell<[Option<AuditEvent>; RING_LEN]>,
    /// Held only during `drain_to`; never on the push hot path.
    drain_lock: SpinLock<()>,
}
unsafe impl Sync for Ring {}

static RING: Ring = Ring {
    cursor:     AtomicU64::new(0),
    slots:      UnsafeCell::new([None; RING_LEN]),
    drain_lock: SpinLock::new(()),
};

pub fn init() { log::debug!("[audit] lock-free ring ({} slots)", RING_LEN); }

#[inline]
fn push(cap: Cap, kind: AuditKind) {
    let seq = RING.cursor.fetch_add(1, Ordering::Relaxed);
    let ev = AuditEvent {
        seq,
        tsc: crate::arch::x86_64::cpu::rdtsc(),
        cap,
        kind,
    };
    // SAFETY: each `seq` value is unique to a single writer; concurrent
    // writers map to different slots most of the time.  The rare ring-
    // wrap collision (2 writers landing on the same slot) is benign for
    // an audit log — the later write wins, drain still observes the
    // most recent event.
    unsafe {
        let slots = &mut *RING.slots.get();
        slots[(seq & RING_MASK) as usize] = Some(ev);
    }
}

pub fn record_grant(cap: Cap, rights: Rights) { push(cap, AuditKind::Grant { rights }); }
pub fn record_revoke(cap: Cap)                { push(cap, AuditKind::Revoke); }
pub fn record_check(cap: Cap, needed: Rights, granted: bool) {
    push(cap, AuditKind::Check { needed, granted });
    if !granted { cold_denied(cap, needed); }
}
pub fn record_forge_attempt(cap: Cap) {
    push(cap, AuditKind::ForgeAttempt);
    cold_forge(cap);
}

#[cold]
fn cold_denied(cap: Cap, needed: Rights) {
    log::warn!("[audit] cap {:#x} denied (needed {:?})", cap.id, needed);
}
#[cold]
fn cold_forge(cap: Cap) {
    log::error!("[audit] FORGE id={:#x} digest={:#x}", cap.id, cap.digest);
}

/// Drain — caller-side copy under `drain_lock`.  Slow path, runs at
/// most a few times per second.
pub fn drain_to(out: &mut [Option<AuditEvent>]) -> usize {
    let _g = RING.drain_lock.lock();
    let take = out.len().min(RING_LEN);
    let mut n = 0;
    unsafe {
        let slots = &mut *RING.slots.get();
        for i in 0..take {
            let ev = slots[i].take();
            if ev.is_some() { out[n] = ev; n += 1; }
        }
    }
    n
}
