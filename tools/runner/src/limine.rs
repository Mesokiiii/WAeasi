//! Embedded Limine bootloader assets.
//!
//! The UEFI loader binary is committed under `bootloader/limine/` and
//! pulled into the runner crate at compile time.  See that directory's
//! `README.md` for provenance and the LICENSE file for upstream
//! redistribution terms (BSD-2-Clause).
//!
//! `LIMINE_CONF` is the on-disk configuration consumed by Limine at
//! boot.  We tell Limine to load the kernel via the multiboot2
//! protocol — the same protocol implemented by `kernel/src/boot/
//! multiboot2.rs`.  No multiboot1 fallback, no Limine-specific kernel
//! changes, no second code path.

/// UEFI x86_64 Limine loader, written to `/EFI/BOOT/BOOTX64.EFI`.
pub const BOOTX64_EFI: &[u8] =
    include_bytes!("../../../bootloader/limine/BOOTX64.EFI");

/// Limine v12 configuration file.
///
/// `timeout: 0` skips the menu and boots straight away — desirable for
/// CI runs.  Set `timeout` to a positive number to interactively pick
/// an entry during local development.
///
/// `protocol: limine` uses Limine's native boot protocol.  This sets up
/// long mode + identity-mapping for the first 4 GiB + higher-half
/// kernel mapping at the linker VMA — exactly what `arch/x86_64/boot.rs`
/// `_start` expects.
///
/// `path: boot():/kernel` resolves to the FAT volume Limine itself
/// booted from (the FAT image `disk.rs` assembles).
pub const LIMINE_CONF: &str = "\
timeout: 0
serial: yes

/WAeasi
    protocol: limine
    path: boot():/kernel
";

/// Where the EFI loader lives inside the FAT volume.
pub const EFI_LOADER_PATH: &[&str] = &["EFI", "BOOT", "BOOTX64.EFI"];

/// File names placed at the FAT root.
pub const KERNEL_FILE_NAME:  &str = "kernel";
pub const CONFIG_FILE_NAME:  &str = "limine.conf";
