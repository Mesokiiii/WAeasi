//! Kernel binary entry point.
//!
//! The init graph is partitioned in two: a **core** path that brings
//! up everything the kernel needs to print structured logs and run
//! its async executor, and an **optional** path (gated by the
//! `full-init` cargo feature) that touches the still-unverified
//! crypto / ACPI / SMP / drivers stack.
//!
//! Goal of the core path: every line printed is a `log::info!()` from
//! a subsystem that has actually been validated end-to-end on QEMU.
#![no_std]
#![no_main]

use waeasi_kernel::{arch, boot, drivers, kernel_main, log_, memory};

#[cfg(feature = "full-init")]
use waeasi_kernel::{
    acpi, admin, crypto, obs, sched, security, sync, wasi, wasm,
};

#[unsafe(no_mangle)]
pub extern "C" fn kernel_entry(boot_info_ptr: usize) -> ! {
    // 1. Earliest output — UART before anything else can fail.
    drivers::serial::init();
    log_::init();

    // 2. GDT + IDT + exception stubs (so any fault from here on
    //    surfaces as a structured `wadbg`-friendly dump rather than a
    //    silent triple-fault).
    arch::init_early();

    log::info!("");
    log::info!("===============================================");
    log::info!("  Hello, World!  WAeasi kernel is alive.");
    log::info!("===============================================");
    log::info!("[main] kernel_entry, boot_info_ptr={:#x}", boot_info_ptr);

    // 3. Boot info from the bootloader.
    let boot_info = boot::parse(boot_info_ptr);
    log::info!("[main] boot: {} regions, cmdline='{}'",
               boot_info.mem_regions.len(), boot_info.cmdline);

    // 4. Memory: physical frames -> paging -> kernel heap -> SAS arena.
    memory::init(boot_info_ptr);

    // 5. APIC + IRQ enable.
    arch::init_late();

    // 6. Per-CPU GS-base.
    arch::x86_64::per_cpu::init(64);

    #[cfg(feature = "full-init")]
    full_init();

    log::info!("[main] core init done; entering executor idle loop.");
    log::info!("[main] (full-init feature is {})",
               if cfg!(feature = "full-init") { "ON" } else { "OFF — verified subsystems only" });

    kernel_main();
}

#[cfg(feature = "full-init")]
fn full_init() {
    security::init();
    obs::init();
    if let Err(e) = crypto::ed25519::self_test() {
        panic!("crypto self-test failed: {}", e);
    }
    let acpi_info = acpi::parse();
    if let Some(madt) = &acpi_info.madt {
        log::info!("[main] ACPI MADT: {} CPUs, {} I/O APICs",
                   madt.cpus.len(), madt.ioapics.len());
    }
    let cpu_count = arch::x86_64::smp::start_aps() as usize;
    sched::executor::Executor::init_pool(cpu_count.max(1));
    sync::init();
    drivers::init();
    wasm::init();
    wasi::init();
    admin::init();
    sched::bootstrap();
}
