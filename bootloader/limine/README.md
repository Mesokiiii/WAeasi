# Limine bootloader (vendored)

This directory ships a pre-compiled UEFI binary of the
[Limine](https://github.com/limine-bootloader/limine) bootloader, used
by `tools/runner` to package the WAeasi kernel into a bootable disk
image.

## Contents

| File | Purpose |
|---|---|
| `BOOTX64.EFI` | Limine UEFI x86_64 loader, embedded in the FAT image at `/EFI/BOOT/BOOTX64.EFI` |
| `LICENSE`     | Upstream BSD-2-Clause license (preserved for redistribution) |

## Provenance

* **Project**: `limine-bootloader/limine`
* **Version**: `v12.3.3`
* **Source**: <https://github.com/limine-bootloader/limine/releases/tag/v12.3.3>
* **Asset**: `limine-binary.tar.gz`
* **Downloaded file inside archive**: `limine-binary/BOOTX64.EFI`
* **License**: BSD-2-Clause (see `LICENSE`)

## Why vendored

Limine is BSD-2-Clause licensed; redistribution of binary builds is
permitted as long as the LICENSE file is preserved.  We vendor the
single 365 KiB binary (instead of downloading at build time) for two
reasons:

1. **Reproducibility** — every checkout produces the same disk image
   without hitting the network.
2. **Offline development** — `cargo krun` works on disconnected
   workstations and CI runners without internet egress.

## Updating

```powershell
# 1. Pick the desired tag at https://github.com/limine-bootloader/limine/releases
$tag = "v12.3.3"
Invoke-WebRequest -UseBasicParsing `
    -Uri "https://github.com/limine-bootloader/limine/releases/download/$tag/limine-binary.tar.gz" `
    -OutFile $env:TEMP\limine.tar.gz
tar -xzf $env:TEMP\limine.tar.gz -C $env:TEMP

# 2. Replace files
Copy-Item "$env:TEMP\limine-binary\BOOTX64.EFI" .\BOOTX64.EFI -Force
Copy-Item "$env:TEMP\limine-binary\LICENSE"     .\LICENSE     -Force

# 3. Bump the version reference in this README and commit.
```

## Why not GRUB / multiboot1 / shim

* GRUB requires `grub-mkrescue` + `xorriso` + `mtools`, which on
  Windows means installing MSYS2 or WSL.
* A handcrafted multiboot1 stub would diverge from the kernel's
  actual boot protocol (multiboot2) and add a second code path to
  maintain.
* Limine speaks the same multiboot2 protocol the kernel already
  implements, runs from a single 365 KiB UEFI binary, and works
  identically on Windows, Linux and macOS hosts.
