// Diagnostic raw-print helpers for the IDT module.  Included via
// `include!` from `idt.rs` so they share its visibility.

#[inline(always)]
fn putb(b: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x3f8u16, in("al") b,
            options(nostack, nomem, preserves_flags),
        );
    }
}
fn puts(s: &str) { for &b in s.as_bytes() { putb(b); } }

fn put_hex_u64(mut v: u64) {
    putb(b'0'); putb(b'x');
    let mut nib = [0u8; 16];
    for i in (0..16).rev() {
        let n = (v & 0xf) as u8;
        nib[i] = if n < 10 { b'0' + n } else { b'a' + n - 10 };
        v >>= 4;
    }
    for n in nib { putb(n); }
}
fn put_hex_u16(v: u16) { put_hex_u64(v as u64); }

fn read_cr2() -> u64 {
    let v: u64;
    unsafe { core::arch::asm!("mov {}, cr2", out(reg) v, options(nomem, nostack)) };
    v
}
fn read_rbp() -> u64 {
    let v: u64;
    unsafe { core::arch::asm!("mov {}, rbp", out(reg) v, options(nomem, nostack)) };
    v
}

/// Walk the rbp frame-pointer chain and emit up to `MAX` return
/// addresses on COM1.  The kernel is built with
/// `-Cforce-frame-pointers=yes` so every Rust callee preserves the
/// `[rbp+8] = saved-RIP, [rbp] = saved-RBP` invariant.  Stops on
/// either: NULL frame, frame outside the kernel higher-half range
/// (caught either by the RBP value or by RIP), or `MAX` frames.
fn backtrace() {
    const MAX: usize = 8;
    const HIGHER_HALF: u64 = 0xFFFF_8000_0000_0000;
    puts("\n  bt:\n");
    let mut rbp = read_rbp();
    for _ in 0..MAX {
        if rbp < HIGHER_HALF || rbp & 0x7 != 0 { break; }
        let saved_rip = unsafe { core::ptr::read_volatile((rbp + 8) as *const u64) };
        let next_rbp  = unsafe { core::ptr::read_volatile(rbp        as *const u64) };
        if saved_rip < HIGHER_HALF { break; }
        puts("    "); put_hex_u64(saved_rip); puts("\n");
        if next_rbp <= rbp { break; }   // also catches NULL
        rbp = next_rbp;
    }
}

/// Common diagnostic for every exception stub: dump frame + halt.
#[cold]
fn dump(name: &str, f: &InterruptFrame, ec: Option<u64>, has_cr2: bool) -> ! {
    puts("\n\n=== EXCEPTION ");
    puts(name);
    puts(" ===\n  RIP = "); put_hex_u64(f.rip);
    puts("\n  CS  = ");     put_hex_u16(f.cs as u16);
    puts("\n  RSP = ");     put_hex_u64(f.rsp);
    puts("\n  SS  = ");     put_hex_u16(f.ss as u16);
    puts("\n  RFL = ");     put_hex_u64(f.rflags);
    if let Some(e) = ec { puts("\n  ERR = "); put_hex_u64(e); }
    if has_cr2          { puts("\n  CR2 = "); put_hex_u64(read_cr2()); }
    backtrace();
    puts("  CPU halted.\n");
    loop {
        unsafe { core::arch::asm!("cli; hlt", options(nomem, nostack)) };
    }
}
