//! Kernel binary entry point — Stage 8 init graph.
//!
//! ```text
//!   serial → log → arch::init_early → boot::parse → memory
//!                → arch::init_late → per_cpu → security
//!                → obs (tracing+metrics) → crypto self-test
//!                → acpi::parse → smp::start_aps → executor::init_pool
//!                → sync → drivers → wasm → wasi → admin → sched
//! ```
#![no_std]
#![no_main]

use waeasi_kernel::{
    acpi, admin, arch, boot, crypto, drivers, kernel_main, log_, memory,
    obs, sched, security, sync, wasi, wasm,
};

#[unsafe(no_mangle)]
pub extern "C" fn kernel_entry(boot_info_ptr: usize) -> ! {
    // 1. Earliest output — UART before anything else can fail.
    drivers::serial::init();
    log_::init();

    // 2. Architecture: GDT + IDT + exceptions.  No allocation yet.
    arch::init_early();

    // 3. Boot info from the bootloader.
    let boot_info = boot::parse(boot_info_ptr);

    // 4. Memory: physical frames → paging → kernel heap → SAS arena.
    memory::init(boot_info_ptr);

    // 5. Architecture: APIC + IRQ enable.
    arch::init_late();

    // 6. Per-CPU GS-base (security needs it for stack canaries).
    arch::x86_64::per_cpu::init(64);

    // 7. Security primitives: capabilities, audit, W⊕X, Spectre, canary.
    security::init();

    // 8. Observability — tracing + metrics registry.  Must precede any
    //    component that wants to emit structured spans.
    obs::init();

    // 9. Cryptographic self-test.  Failure here aborts boot — Ed25519
    //    must be correct before any TLS handshake completes.
    if let Err(e) = crypto::ed25519::self_test() {
        panic!("crypto self-test failed: {}", e);
    }

    // 10. ACPI walk — populates topology + HPET + MCFG.
    let acpi_info = acpi::parse();
    if let Some(madt) = &acpi_info.madt {
        log::info!("[main] ACPI MADT: {} CPUs, {} I/O APICs",
                   madt.cpus.len(), madt.ioapics.len());
    }

    // 11. SMP startup + executor pool.
    let cpu_count = arch::x86_64::smp::start_aps() as usize;
    sched::executor::Executor::init_pool(cpu_count.max(1));

    // 12. Sync layer — Once cells warm-up.
    sync::init();

    // 13. Drivers + Wasm engine + WASI.
    drivers::init();
    wasm::init();
    wasi::init();

    // 14. Admin endpoint (waeasictl protocol over TCP).
    admin::init();

    // 15. Spawn the boot service that loads `components/`.
    sched::bootstrap();

    log::info!("[main] boot_info: {} regions, cmdline='{}'",
               boot_info.mem_regions.len(), boot_info.cmdline);

    // 16. Never returns — runs the async executor forever.
    kernel_main();
}
