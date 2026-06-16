//! FAT32 disk-image builder.
//!
//! Assembles a bootable raw image entirely in user-space, with no
//! external tooling.  `fatfs` provides the filesystem driver; Windows /
//! Linux / macOS hosts produce byte-identical output.
//!
//! Layout written into the image:
//!
//! ```text
//!   /EFI/BOOT/BOOTX64.EFI    Limine UEFI loader
//!   /limine.conf             Boot configuration
//!   /kernel                  Multiboot2-headered kernel ELF
//! ```
//!
//! The default size (32 MiB) leaves comfortable head-room for future
//! resources (initrd, embedded components, snapshots).  Tune via
//! `WAEASI_DISK_BYTES` when needed; minimum permitted by the FAT32
//! geometry is ~33 MiB.

use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use fatfs::{FileSystem, FormatVolumeOptions, FsOptions};

use crate::limine;

/// 32 MiB — the smallest size that round-trips cleanly through FAT32
/// formatting on both Windows and Linux hosts.
pub const DEFAULT_SIZE_BYTES: u64 = 32 * 1024 * 1024;

/// Assemble the bootable image.  Returns the path to the resulting raw
/// disk image (always written next to the kernel ELF).
pub fn build(kernel_elf: &Path) -> io::Result<PathBuf> {
    let size = override_size_or(DEFAULT_SIZE_BYTES);
    let out_dir = kernel_elf.parent().unwrap_or_else(|| Path::new("."));
    let img_path = out_dir.join("disk.img");

    create_blank(&img_path, size)?;
    format_volume(&img_path)?;
    populate(&img_path, kernel_elf)?;
    Ok(img_path)
}

fn override_size_or(default: u64) -> u64 {
    std::env::var("WAEASI_DISK_BYTES")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v >= 33 * 1024 * 1024)
        .unwrap_or(default)
}

fn create_blank(path: &Path, size: u64) -> io::Result<()> {
    let f = OpenOptions::new()
        .create(true).read(true).write(true).truncate(true)
        .open(path)?;
    f.set_len(size)?;
    Ok(())
}

fn format_volume(path: &Path) -> io::Result<()> {
    let mut f = OpenOptions::new().read(true).write(true).open(path)?;
    let opts = FormatVolumeOptions::new()
        .volume_label(*b"WAEASI     "); // 11 bytes, FAT short label
    fatfs::format_volume(&mut f, opts).map_err(io::Error::other)?;
    Ok(())
}

fn populate(image: &Path, kernel_elf: &Path) -> io::Result<()> {
    let f = OpenOptions::new().read(true).write(true).open(image)?;
    let fs = FileSystem::new(f, FsOptions::new()).map_err(io::Error::other)?;
    {
        let root = fs.root_dir();
        write_efi_loader(&root, limine::BOOTX64_EFI)?;
        write_root_file(&root, limine::CONFIG_FILE_NAME, limine::LIMINE_CONF.as_bytes())?;
        write_root_file(&root, limine::KERNEL_FILE_NAME, &read_all(kernel_elf)?)?;
    }
    fs.unmount().map_err(io::Error::other)?;
    Ok(())
}

fn write_efi_loader<IO: fatfs::ReadWriteSeek>(
    root: &fatfs::Dir<'_, IO>,
    bytes: &[u8],
) -> io::Result<()> {
    // Walk path components, creating each directory exactly once.
    let mut cur = root.clone();
    let comps = limine::EFI_LOADER_PATH;
    if comps.len() < 2 {
        return Err(io::Error::other("EFI_LOADER_PATH too short"));
    }
    for dir in &comps[..comps.len() - 1] {
        cur = cur.create_dir(dir).map_err(io::Error::other)?;
    }
    let leaf = comps.last().expect("non-empty path");
    let mut f = cur.create_file(leaf).map_err(io::Error::other)?;
    f.truncate().map_err(io::Error::other)?;
    f.write_all(bytes)?;
    f.flush()?;
    Ok(())
}

fn write_root_file<IO: fatfs::ReadWriteSeek>(
    root: &fatfs::Dir<'_, IO>,
    name: &str,
    bytes: &[u8],
) -> io::Result<()> {
    let mut f = root.create_file(name).map_err(io::Error::other)?;
    f.truncate().map_err(io::Error::other)?;
    f.write_all(bytes)?;
    f.flush()?;
    Ok(())
}

fn read_all(path: &Path) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();
    OpenOptions::new().read(true).open(path)?.read_to_end(&mut buf)?;
    Ok(buf)
}
