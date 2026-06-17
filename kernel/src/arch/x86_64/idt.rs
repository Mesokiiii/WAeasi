//! Interrupt Descriptor Table — hand-rolled, no `x86_64`-crate dependency.
//!
//! The crate-provided `InterruptDescriptorTable` was observed to encode
//! handler offsets incorrectly in our build (#UD landed on `pc=0x3`),
//! which is why this module installs entries by direct field stores.
//! The resulting code is small, auditable and has no hidden codegen path.
//!
//! Exception stubs deliberately do **not** use `core::fmt::write` /
//! `panic!` — a fault inside the formatter would otherwise recurse and
//! deadlock.  All diagnostics go to COM1 via `out 0x3f8, al`.
use core::sync::atomic::{AtomicBool, Ordering};

/// One IDT entry in long mode: 16 bytes.
#[repr(C, packed)]
#[derive(Copy, Clone)]
struct Entry {
    offset_low:  u16,
    selector:    u16,
    ist:         u8,
    type_attr:   u8,
    offset_mid:  u16,
    offset_high: u32,
    zero:        u32,
}

const ZERO: Entry = Entry {
    offset_low: 0, selector: 0, ist: 0, type_attr: 0,
    offset_mid: 0, offset_high: 0, zero: 0,
};

#[repr(C, packed)]
struct Idtr { limit: u16, base: u64 }

/// 64-bit interrupt gate, present, DPL=0.
const GATE: u8 = 0x8E;
/// Code segment selector left in place by `boot32._long_mode_start`.
const KERNEL_CS: u16 = 0x08;

static INITIALIZED: AtomicBool = AtomicBool::new(false);
static mut IDT: [Entry; 256]   = [ZERO; 256];

/// Write one IDT entry by individual field stores.  Inlined `mov`s
/// through inline asm prevent the compiler from collapsing the seven
/// stores into a single SIMD-style 16-byte write.
#[inline(never)]
fn write_entry(vec: u8, handler: u64) {
    #[allow(static_mut_refs)]
    let p   = unsafe { (IDT.as_mut_ptr() as *mut u8).add(vec as usize * 16) } as usize;
    let lo  = (handler & 0xFFFF) as u16;
    let mid = ((handler >> 16) & 0xFFFF) as u16;
    let hi  = ((handler >> 32) & 0xFFFF_FFFF) as u32;
    unsafe {
        core::arch::asm!(
            "mov word ptr [{p} + 0],   {lo:x}",
            "mov word ptr [{p} + 2],   {sel:x}",
            "mov byte ptr [{p} + 4],   0",
            "mov byte ptr [{p} + 5],   {ga}",
            "mov word ptr [{p} + 6],   {mid:x}",
            "mov dword ptr [{p} + 8],  {hi:e}",
            "mov dword ptr [{p} + 12], 0",
            p   = in(reg) p,
            lo  = in(reg) lo,
            sel = in(reg) KERNEL_CS as u16,
            ga  = in(reg_byte) GATE,
            mid = in(reg) mid,
            hi  = in(reg) hi,
        );
    }
}

pub fn set(vec: u8, handler: extern "x86-interrupt" fn(InterruptFrame)) {
    write_entry(vec, handler as usize as u64);
}
pub fn set_ec(vec: u8, handler: extern "x86-interrupt" fn(InterruptFrame, u64)) {
    write_entry(vec, handler as usize as u64);
}

/// Install exception stubs and load IDTR.
pub fn init() {
    if INITIALIZED.swap(true, Ordering::AcqRel) {
        return;
    }

    set(0,    stub_de);   set(1,  stub_db);   set(2,  stub_nmi);
    set(3,    stub_bp);   set(4,  stub_of);   set(5,  stub_br);
    set(6,    stub_ud);   set(7,  stub_nm);
    set_ec(8, stub_df);
    set_ec(10, stub_ts);  set_ec(11, stub_np);
    set_ec(12, stub_ss);  set_ec(13, stub_gp);
    set_ec(14, stub_pf);
    set(16, stub_mf);     set_ec(17, stub_ac); set(19, stub_xm);
    set(20, stub_ve);

    let idtr = Idtr {
        limit: (core::mem::size_of::<[Entry; 256]>() - 1) as u16,
        #[allow(static_mut_refs)]
        base:  unsafe { IDT.as_ptr() as u64 },
    };
    unsafe { core::arch::asm!("lidt [{}]", in(reg) &idtr, options(readonly, nostack)) };

    // Mask every legacy 8259 PIC line so the BIOS-installed IRQ
    // mapping (timer @ vector 0x08, etc — overlapping our exception
    // vectors) cannot deliver a spurious interrupt that would land
    // inside an exception stub.  The kernel uses the LAPIC + IO-APIC
    // for real interrupt delivery and re-enables only the lines it
    // wants from there.
    unsafe {
        core::arch::asm!(
            "out 0xa1, al",
            "out 0x21, al",
            in("al") 0xffu8,
            options(nostack, nomem, preserves_flags),
        );
    }
}

/// Layout pushed by the CPU on every interrupt (no error code).
#[repr(C)]
pub struct InterruptFrame { rip: u64, cs: u64, rflags: u64, rsp: u64, ss: u64 }

extern "x86-interrupt" fn stub_de (f: InterruptFrame)            { dump("#DE  divide error",       &f, None,    false); }
extern "x86-interrupt" fn stub_db (f: InterruptFrame)            { dump("#DB  debug",               &f, None,    false); }
extern "x86-interrupt" fn stub_nmi(f: InterruptFrame)            { dump("#NMI non-maskable",        &f, None,    false); }
extern "x86-interrupt" fn stub_bp (f: InterruptFrame)            { dump("#BP  breakpoint",          &f, None,    false); }
extern "x86-interrupt" fn stub_of (f: InterruptFrame)            { dump("#OF  overflow",            &f, None,    false); }
extern "x86-interrupt" fn stub_br (f: InterruptFrame)            { dump("#BR  bound range",         &f, None,    false); }
extern "x86-interrupt" fn stub_ud (f: InterruptFrame)            { dump("#UD  invalid opcode",      &f, None,    false); }
extern "x86-interrupt" fn stub_nm (f: InterruptFrame)            { dump("#NM  device not avail",    &f, None,    false); }
extern "x86-interrupt" fn stub_mf (f: InterruptFrame)            { dump("#MF  x87 fp",              &f, None,    false); }
extern "x86-interrupt" fn stub_xm (f: InterruptFrame)            { dump("#XM  SIMD fp",             &f, None,    false); }
extern "x86-interrupt" fn stub_ve (f: InterruptFrame)            { dump("#VE  virtualization",      &f, None,    false); }
extern "x86-interrupt" fn stub_ts (f: InterruptFrame, ec: u64)   { dump("#TS  invalid TSS",         &f, Some(ec), false); }
extern "x86-interrupt" fn stub_np (f: InterruptFrame, ec: u64)   { dump("#NP  segment not present", &f, Some(ec), false); }
extern "x86-interrupt" fn stub_ss (f: InterruptFrame, ec: u64)   { dump("#SS  stack-segment fault", &f, Some(ec), false); }
extern "x86-interrupt" fn stub_gp (f: InterruptFrame, ec: u64)   { dump("#GP  general protection",  &f, Some(ec), false); }
extern "x86-interrupt" fn stub_ac (f: InterruptFrame, ec: u64)   { dump("#AC  alignment check",     &f, Some(ec), false); }
extern "x86-interrupt" fn stub_pf (f: InterruptFrame, ec: u64)   { dump("#PF  page fault",          &f, Some(ec), true);  }
extern "x86-interrupt" fn stub_df (f: InterruptFrame, ec: u64)   { dump("#DF  double fault",     &f, Some(ec), false); }

include!("idt_dump.rs");
