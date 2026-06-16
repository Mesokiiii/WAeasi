//! Boot trampoline for x86_64 — pure higher-half.
//!
//! Contract with the bootloader (Limine v6 / Multiboot2 + GRUB shim):
//!   * long mode is on,
//!   * the first 4 GiB of physical RAM are identity-mapped **and** mirrored
//!     at `DIRECT_MAP_BASE`,
//!   * the kernel ELF is mapped at its linker-script VMA (higher half),
//!   * the bootloader jumps to `_start` at its higher-half virtual address,
//!   * `rdi` carries the bootloader info pointer (System V AMD64 ABI).
//!
//! `_start` therefore runs with RIP already in the higher half, so every
//! intra-kernel `call rel32` fits.  No low-half trampoline is needed.
use core::arch::naked_asm;

/// 64 KiB initial stack — enough to bring up `memory::heap`.
#[repr(align(16))]
struct BootStack([u8; 64 * 1024]);
#[unsafe(no_mangle)]
static mut BOOT_STACK: BootStack = BootStack([0; 64 * 1024]);

extern "C" {
    fn kernel_entry(boot_info_ptr: usize) -> !;
    static __bss_start: u8;
    static __bss_end: u8;
}

/// Real entry point.  `#[naked]` + `naked_asm!` per Rust 2024 ABI.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _start() -> ! {
    naked_asm!(
        "lea rsp, [rip + {stack} + {stack_size}]",
        "xor rbp, rbp",
        "mov r12, rdi",
        "call {bss_clear}",
        "mov rdi, r12",
        "call {entry}",
        "2: cli",
        "   hlt",
        "   jmp 2b",
        stack       = sym BOOT_STACK,
        stack_size  = const 64 * 1024,
        bss_clear   = sym clear_bss,
        entry       = sym kernel_entry,
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
