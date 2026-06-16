//! Physical frame allocator.
//!
//! Hot path:
//!   * `alloc_frame()`  — try free-list (single CAS retry) → bump cursor
//!                        (single `fetch_add`).  Lock-free, wait-free
//!                        in the bump branch.
//!   * `free_frame(f)`  — single CAS retry to push onto a tagged-pointer
//!                        Treiber stack.
//!
//! The free-list head is a 64-bit packed `(version, addr_in_low_48b)`
//! to defeat ABA: any time the head moves, the version increments,
//! invalidating stale CAS attempts that loaded the old `(addr, ver)`.
//! 16-bit version space; on a 4 GHz CPU pushing 10 ns/op, wraparound
//! takes 650 µs — far more than any single CAS retry ever takes.
use core::sync::atomic::{AtomicU64, Ordering};

use super::address::{PhysAddr, PAGE_SIZE_4K};
use super::paging::{phys_to_virt, virt_to_phys, DIRECT_MAP_BASE};

/// Cache-line-padded bump cursor.
#[repr(align(64))]
struct Bump { next: AtomicU64, end: AtomicU64 }

#[repr(C)]
struct FreeNode { next_addr: AtomicU64 } // 48-bit virt addr; 16 bits version

static BUMP: Bump = Bump { next: AtomicU64::new(0), end: AtomicU64::new(0) };
/// Packed head: `(version << 48) | (virt_addr & 0xFFFF_FFFF_FFFF)`.
static FREE_HEAD: AtomicU64 = AtomicU64::new(0);

const ADDR_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
const VER_SHIFT: u32 = 48;

#[inline]
fn pack(addr: u64, version: u16) -> u64 {
    (addr & ADDR_MASK) | ((version as u64) << VER_SHIFT)
}
#[inline]
fn unpack(packed: u64) -> (u64, u16) {
    (packed & ADDR_MASK, (packed >> VER_SHIFT) as u16)
}

/// 4 KiB physical frame.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Frame(pub PhysAddr);

impl Frame {
    pub fn containing(addr: PhysAddr) -> Self { Self(addr.align_down(PAGE_SIZE_4K)) }
    pub fn next(self) -> Self {
        Self(PhysAddr::new(self.0.as_usize() + PAGE_SIZE_4K as usize))
    }
}

pub fn init(_boot_info_ptr: usize) {
    let start = 16 * 1024 * 1024_u64;
    let end   = start + 64 * 1024 * 1024;
    BUMP.next.store(start, Ordering::Release);
    BUMP.end .store(end,   Ordering::Release);
    FREE_HEAD.store(0,     Ordering::Release);
    log::info!("[frame_alloc] {} MiB @ {:#x}..{:#x} (ABA-safe Treiber)",
               (end - start) / (1024 * 1024), start, end);
}

/// Allocate a single 4 KiB frame.
pub fn alloc_frame() -> Option<Frame> {
    if let Some(f) = pop_free() { return Some(f); }
    let next = BUMP.next.fetch_add(PAGE_SIZE_4K, Ordering::Relaxed);
    let end  = BUMP.end.load(Ordering::Relaxed);
    if next + PAGE_SIZE_4K > end {
        BUMP.next.store(end, Ordering::Relaxed);
        return cold_oom();
    }
    Some(Frame(PhysAddr::new(next as usize)))
}

/// Push a frame onto the free list (tagged-pointer CAS).
pub fn free_frame(frame: Frame) {
    let v_addr = phys_to_virt(frame.0).as_u64();
    debug_assert!(v_addr & !ADDR_MASK == DIRECT_MAP_BASE as u64 & !ADDR_MASK);
    let node = v_addr as *mut FreeNode;

    let mut cur = FREE_HEAD.load(Ordering::Acquire);
    loop {
        // Write our own `next_addr` first (carries old packed head).
        unsafe { (*node).next_addr.store(cur, Ordering::Relaxed); }
        let (_old_addr, old_ver) = unpack(cur);
        let new = pack(v_addr, old_ver.wrapping_add(1));
        match FREE_HEAD.compare_exchange_weak(cur, new, Ordering::Release, Ordering::Acquire) {
            Ok(_) => return,
            Err(actual) => cur = actual,
        }
    }
}

fn pop_free() -> Option<Frame> {
    use super::address::VirtAddr;
    loop {
        let head = FREE_HEAD.load(Ordering::Acquire);
        let (addr, ver) = unpack(head);
        if addr == 0 { return None; }
        // Recover the canonical higher-half pointer.  The address
        // stored is direct-map; canonicalize the upper bits.
        let canon = canonicalize(addr);
        let next_packed = unsafe { (*(canon as *const FreeNode)).next_addr.load(Ordering::Acquire) };
        let new = {
            let (_a, _v) = unpack(next_packed);
            // version increments globally: any pop bumps the new head's tag
            pack(unpack(next_packed).0, ver.wrapping_add(1))
        };
        if FREE_HEAD
            .compare_exchange_weak(head, new, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let phys = virt_to_phys(VirtAddr::new(canon as usize))?;
            return Some(Frame(phys));
        }
    }
}

#[inline]
fn canonicalize(low48: u64) -> u64 {
    // Direct-map addresses are 0xFFFF_8000_0000_0000.. — sign-extend
    // bit 47 to get a valid x86_64 canonical pointer.
    if low48 & (1 << 47) != 0 { low48 | 0xFFFF_0000_0000_0000 } else { low48 }
}

#[cold]
fn cold_oom() -> Option<Frame> { log::error!("[frame_alloc] OOM"); None }
