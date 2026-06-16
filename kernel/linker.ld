/* =============================================================================
 * WAeasi linker script — pure higher-half x86_64 kernel.
 *
 * Boot contract (Limine v6):
 *   * the bootloader switches the CPU to long mode,
 *   * sets up identity-mapping for the first 4 GiB,
 *   * AND maps the kernel image starting at KERNEL_OFFSET (higher half),
 *   * jumps to `_start` at its higher-half virtual address.
 *
 * Result: every symbol — including `_start` — has a higher-half VMA, so
 * intra-kernel `call rel32`s always fit. There is **no** low-half trampoline.
 * The 1 MiB physical load address still applies for the LMA so the ELF can
 * be loaded by GRUB/Limine.
 * ========================================================================== */
ENTRY(_start)

KERNEL_OFFSET = 0xFFFFFFFF80000000;
PHYS_BASE     = 0x100000;

SECTIONS {
    . = KERNEL_OFFSET + PHYS_BASE;

    .text ALIGN(4K) : AT(ADDR(.text) - KERNEL_OFFSET) {
        KEEP(*(.multiboot_header))
        KEEP(*(.boot.text))
        *(.text .text.*)
    }

    .rodata ALIGN(4K) : AT(ADDR(.rodata) - KERNEL_OFFSET) {
        *(.rodata .rodata.*)
    }

    .data ALIGN(4K) : AT(ADDR(.data) - KERNEL_OFFSET) {
        KEEP(*(.boot.data))
        *(.data .data.*)
    }

    .bss ALIGN(4K) : AT(ADDR(.bss) - KERNEL_OFFSET) {
        __bss_start = .;
        *(COMMON)
        *(.bss .bss.*)
        __bss_end = .;
    }

    __kernel_end = .;

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
        *(.comment)
    }
}
