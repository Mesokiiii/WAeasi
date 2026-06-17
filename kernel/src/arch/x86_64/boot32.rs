//! Multiboot1 boot trampoline.
//!
//! `qemu -kernel ELF`, GRUB and Limine-BIOS all speak Multiboot1 and
//! hand off in 32-bit protected mode with paging disabled.  The
//! kernel is linked at the higher-half VMA, so this shim runs first
//! in low memory and:
//!
//!   1. Builds 4-level page tables with three views of the first GiB:
//!        * identity        (VA = PA)              — for the 32-bit shim
//!        * direct map      (`0xFFFF_8000_…`)     — `paging::phys_to_virt`
//!        * higher half     (`0xFFFF_FFFF_8000_…`) — kernel image
//!   2. Enables PAE + long mode + paging.
//!   3. Sets `CR0.MP|NE`, `CR4.OSFXSR|OSXMMEXCPT`, `EFER.NXE` so SSE
//!      codegen does not `#UD` and bit-63 of PT entries is treated as
//!      no-execute (not reserved).
//!   4. Loads a 64-bit GDT and far-jumps to a 64-bit stub that puts
//!      the Multiboot info pointer in `rdi` and jumps to `_start`.
use core::arch::global_asm;

global_asm!(
r#"
.intel_syntax noprefix

.section .multiboot_header, "a"
.align 4
.set MB_MAGIC,    0x1BADB002
.set MB_FLAGS,    0
.set MB_CHECKSUM, -(MB_MAGIC + MB_FLAGS)
mb_header:
    .long MB_MAGIC
    .long MB_FLAGS
    .long MB_CHECKSUM

.section .boot32, "ax"
.code32
.global _mb_start
.type _mb_start, @function
_mb_start:
    cli
    cld
    // ebx already holds the Multiboot info pointer; it survives every
    // CR/MSR write below.  The 64-bit stub copies it into rdi.
    lea esp, [_boot32_stack_top]

    // PML4[0] / [256] -> PDPT_LO    ; PML4[511] -> PDPT_HI
    // PDPT_LO[0]      -> PD         ; PDPT_HI[510] -> PD
    // PD[0..511] = 2-MiB pages covering 0..1 GiB (P|W|PS).
    lea eax, [_pdpt_lo]
    or  eax, 0x3
    mov [_pml4 + 0*8],   eax
    mov dword ptr [_pml4 + 0*8 + 4],   0
    mov [_pml4 + 256*8], eax
    mov dword ptr [_pml4 + 256*8 + 4], 0

    lea eax, [_pdpt_hi]
    or  eax, 0x3
    mov [_pml4 + 511*8], eax
    mov dword ptr [_pml4 + 511*8 + 4], 0

    lea eax, [_pd]
    or  eax, 0x3
    mov [_pdpt_lo + 0*8],   eax
    mov dword ptr [_pdpt_lo + 0*8 + 4],   0
    mov [_pdpt_hi + 510*8], eax
    mov dword ptr [_pdpt_hi + 510*8 + 4], 0

    xor edi, edi
    mov eax, 0x00000083                 // page 0: addr=0, P|W|PS
.Lfill_pd:
    mov [_pd + edi*8], eax
    mov dword ptr [_pd + edi*8 + 4], 0
    add eax, 0x00200000
    inc edi
    cmp edi, 512
    jb  .Lfill_pd

    lea eax, [_pml4]
    mov cr3, eax

    mov eax, cr4
    or  eax, (1 << 5) | (1 << 9) | (1 << 10)    // PAE | OSFXSR | OSXMMEXCPT
    mov cr4, eax

    mov eax, cr0
    or  eax, (1 << 1) | (1 << 5)                // MP | NE
    and eax, 0xFFFFFFFB                         // clear EM
    mov cr0, eax

    mov ecx, 0xC0000080
    rdmsr
    or  eax, (1 << 8) | (1 << 11)               // EFER.LME | NXE
    wrmsr

    mov eax, cr0
    or  eax, (1 << 31) | (1 << 0)               // PG | PE
    mov cr0, eax

    lgdt [_gdt64_ptr]
    ljmp 0x08, offset _long_mode_start

.section .boot64, "ax"
.code64
.type _long_mode_start, @function
_long_mode_start:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov rdi, rbx                                // Multiboot info pointer
    movabs rax, offset _start
    jmp rax

.section .boot.data, "aw"
.align 4096
_pml4:    .skip 4096
.align 4096
_pdpt_lo: .skip 4096
.align 4096
_pdpt_hi: .skip 4096
.align 4096
_pd:      .skip 4096

.align 16
_gdt64:
    .quad 0
    .quad 0x00AF9A000000FFFF                    // 0x08: 64-bit code
    .quad 0x00AF92000000FFFF                    // 0x10: 64-bit data
_gdt64_end:

.align 8
_gdt64_ptr:
    .word _gdt64_end - _gdt64 - 1
    .quad _gdt64

.align 16
_boot32_stack:
    .skip 4096
_boot32_stack_top:
"#);
