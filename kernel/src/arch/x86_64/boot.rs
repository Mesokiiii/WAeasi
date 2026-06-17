//! Boot trampoline for x86_64 — pure higher-half.
//!
//! Contract with `arch::x86_64::boot32` (Multiboot1 path) or with a
//! Limine v6 / Multiboot2 + GRUB shim (alternate path):
//!   * long mode is on,
//!   * the first GiB of physical RAM is identity-mapped, mirrored at
//!     `DIRECT_MAP_BASE`, and mirrored at the kernel's higher-half VMA,
//!   * the bootloader / shim jumps to `_start` at its higher-half virtual
//!     address,
//!   * `rdi` carries the bootloader info pointer (System V AMD64 ABI).
//!
//! The boot stack is reserved by the linker as a dedicated section
//! that sits **after** `.bss`, so `clear_bss` cannot scribble on it,
//! and so we can load `rsp` via `movabs` against the absolute symbol
//! `__kernel_stack_top` (which is exact regardless of how far it sits
//! from `_start`).
use core::arch::naked_asm;

extern "C" {
    fn kernel_entry(boot_info_ptr: usize) -> !;
    static __bss_start: u8;
    static __bss_end: u8;
    static __kernel_stack_top: u8;
}

/// Real entry point.  `#[naked]` + `naked_asm!` per Rust 2024 ABI.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        "movabs rsp, offset {stack_top}",
        "xor rbp, rbp",
        "mov r12, rdi",
        "call {bss_clear}",
        "mov rdi, r12",
        "call {entry}",
        "2: cli",
        "   hlt",
        "   jmp 2b",
        stack_top = sym __kernel_stack_top,
        bss_clear = sym clear_bss,
        entry     = sym kernel_entry,
    );
}

/// Zero-initialize `.bss`.  Word-sized stores + tail loop.
#[unsafe(no_mangle)]
unsafe extern "C" fn clear_bss() {
    let start = &__bss_start as *const u8 as usize;
    let end   = &__bss_end   as *const u8 as usize;
    let mut p = start as *mut u8;

    while (p as usize) & 7 != 0 && (p as usize) < end {
        core::ptr::write_volatile(p, 0);
        p = p.add(1);
    }
    let qend = (end & !7) as *mut u64;
    let mut q = p as *mut u64;
    while q < qend {
        core::ptr::write_volatile(q, 0);
        q = q.add(1);
    }
    let mut p = q as *mut u8;
    while (p as usize) < end {
        core::ptr::write_volatile(p, 0);
        p = p.add(1);
    }
}
