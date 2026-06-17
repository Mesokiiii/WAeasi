/* =============================================================================
 * WAeasi linker script.
 *
 * The kernel is a higher-half x86_64 image (VMA = `KERNEL_OFFSET +
 * PHYS_BASE`).  A small low-half boot trampoline (`.boot32` / `.boot64`)
 * runs first in 32-bit protected mode, sets up long mode + paging, and
 * trampolines into the higher-half `_start`.  After that point every
 * intra-kernel symbol fits in a `call rel32`.
 *
 * The boot region reserves 64 KiB at PHYS_BASE for the trampoline,
 * its page tables, GDT and stack.  The higher-half image starts at
 * `PHYS_BASE + BOOT_RESERVE` (LMA) / `KERNEL_OFFSET + PHYS_BASE +
 * BOOT_RESERVE` (VMA).  Boot sections deliberately have VMA == LMA so
 * 32-bit code can address them with plain absolute offsets.
 * ========================================================================== */
ENTRY(_mb_start)

KERNEL_OFFSET = 0xFFFFFFFF80000000;
PHYS_BASE     = 0x100000;
BOOT_RESERVE  = 0x20000;            /* 128 KiB for trampoline + tables + stack */

SECTIONS {
    /* -------------------------------------------------------------- *
     *  Low-memory boot trampoline (VMA == LMA)                       *
     * -------------------------------------------------------------- */
    . = PHYS_BASE;

    .boot32 ALIGN(8) : {
        KEEP(*(.multiboot_header))
        KEEP(*(.boot32))
    }

    .boot64 ALIGN(8) : {
        KEEP(*(.boot64))
    }

    .boot.data ALIGN(4K) : {
        KEEP(*(.boot.data))
    }

    /* -------------------------------------------------------------- *
     *  Higher-half kernel — fixed start past the reserved boot area  *
     * -------------------------------------------------------------- */
    . = KERNEL_OFFSET + PHYS_BASE + BOOT_RESERVE;

    .requests ALIGN(8) : AT(ADDR(.requests) - KERNEL_OFFSET) {
        KEEP(*(.requests_start_marker))
        KEEP(*(.requests))
        KEEP(*(.requests_end_marker))
    }

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_OFFSET) {
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_OFFSET) {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_OFFSET) {
        *(.data .data.*)
    }

    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_OFFSET) {
        __bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        __bss_end = .;
    }

    /* ------------------------------------------------------------- *
     *  Dedicated boot stack — placed AFTER `.bss` so `clear_bss`    *
     *  cannot zero it, and exposed as a linker symbol so `_start`   *
     *  can load `rsp` via `movabs` against an absolute VMA.         *
     * ------------------------------------------------------------- */
    .stack ALIGN(4K) : AT(ADDR(.stack) - KERNEL_OFFSET) {
        __kernel_stack_bottom = .;
        . = . + 64K;
        __kernel_stack_top    = .;
    }

    __kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
        *(.comment)
    }
}
