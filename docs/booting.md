# Booting WAeasi

WAeasi targets bare metal, but day-to-day development happens inside
QEMU.  The kernel is built for the custom `x86_64-waeasi` target (see
[`targets/x86_64-waeasi.json`](../targets/x86_64-waeasi.json)) and
booted via the [Limine](https://github.com/limine-bootloader/limine)
bootloader using its native protocol — Limine sets up long mode, the
identity map for the first 4 GiB and the higher-half mapping at the
kernel's linker VMA before jumping to `_start`, matching the contract
in `kernel/src/arch/x86_64/boot.rs`.

The build pipeline is fully automated by `cargo krun`:

```
cargo kbuild         →  target/x86_64-waeasi/release/waeasi  (kernel ELF)
cargo krun           →  builds + assembles disk image + spawns QEMU
```

## Pipeline

```
       ┌──────────────────────────┐
       │  cargo kbuild            │   custom x86_64-waeasi target
       │  (-Z build-std=core,...) │   produces a 64-bit ELF with the
       └────────────┬─────────────┘   .requests / .text / etc layout
                    │
                    ▼
 ┌──────────────────────────────────────────────────┐
 │  tools/runner    (Rust binary)                   │
 │                                                  │
 │  1. Build a 32 MiB FAT32 image with `fatfs`      │
 │  2. /EFI/BOOT/BOOTX64.EFI   ← Limine UEFI loader │
 │  3. /limine.conf            ← boot config        │
 │  4. /kernel                 ← the ELF            │
 │                                                  │
 │  No external utilities (xorriso, mtools,         │
 │  grub-mkrescue, limine-deploy) required.         │
 └────────────────────┬─────────────────────────────┘
                      │
                      ▼
 ┌──────────────────────────────────────────────────┐
 │  qemu-system-x86_64                              │
 │   -machine q35    -m 256                         │
 │   -drive if=pflash  edk2-x86_64-code.fd          │
 │   -drive if=pflash  ovmf-vars.fd  (per-run)      │
 │   -drive format=raw disk.img                     │
 │   -serial stdio   -display none                  │
 └──────────────────────────────────────────────────┘
```

## Why Limine

Limine is the only piece of the toolchain that is not pure Rust — and
even there we vendor the prebuilt `BOOTX64.EFI` (BSD-2-Clause) directly
in `bootloader/limine/`.  Reasons we use it instead of GRUB:

* GRUB needs `grub-mkrescue` + `xorriso` + `mtools`, which on Windows
  forces an MSYS2 / WSL install.
* Limine speaks the same multiboot2 protocol as GRUB, plus its own
  native protocol that already sets up long mode + higher-half mapping
  for the kernel.
* The whole bootloader is a single 365 KiB UEFI binary — no install
  step, no MBR patching, no special FAT structure.

Limine is loaded by the host firmware (edk2 / OVMF) as a regular
`/EFI/BOOT/BOOTX64.EFI` UEFI application; it then reads `/limine.conf`,
loads the kernel, and hands off in long mode.

## Boot protocol contract

`kernel/src/boot/limine.rs` publishes three Limine requests in the
`.requests` linker section:

| Request | What it asks for |
|---|---|
| `BaseRevision::with_revision(2)` | Limine v6 protocol revision 2 |
| `MemmapRequest`                  | Physical memory map |
| `HhdmRequest`                    | Higher-half direct-map offset |

Two markers (`RequestsStartMarker` / `RequestsEndMarker`) bound the
search region so Limine can find requests without parsing the whole
ELF.  The linker script `kernel/linker.ld` keeps the section alive
with `KEEP(*(.requests*))`.

After Limine completes its hand-off, `_start` (a `#[naked]` 64-bit
function) runs at the higher-half VMA, sets up the boot stack, zeroes
`.bss`, and tail-calls into `kernel_entry(rdi)`.  `boot::parse()`
then dispatches to `boot::limine::try_parse()` first, falling back to
multiboot2 parsing for environments where Limine's MB2 mode is used.

## Toolchain

| Tool      | Purpose                            | How to obtain on Windows                         |
|-----------|------------------------------------|--------------------------------------------------|
| Rust nightly + `rust-src` + `llvm-tools-preview` | builds the kernel | `rustup show` (toolchain pinned in `rust-toolchain.toml`) |
| QEMU 8.2+                  | runs the kernel | `winget install SoftwareFreedomConservancy.QEMU` |
| Limine UEFI binary         | bootloader      | vendored at `bootloader/limine/BOOTX64.EFI` |
| edk2 OVMF firmware         | UEFI firmware   | ships with QEMU at `share/edk2-x86_64-code.fd` |

## Environment overrides

The runner reads three optional environment variables:

| Variable             | Default                                                      | Effect |
|----------------------|--------------------------------------------------------------|--------|
| `WAEASI_OVMF`        | auto-detected (`edk2-x86_64-code.fd` near `qemu-system-x86_64`) | force a specific UEFI firmware path |
| `WAEASI_OVMF_VARS`   | auto-detected (`edk2-i386-vars.fd`)                          | template for the per-run UEFI variables flash |
| `WAEASI_DISK_BYTES`  | `33554432` (32 MiB)                                          | size of the FAT32 image |
| `WAEASI_QEMU_ARGS`   | unset                                                        | extra flags appended to the QEMU command (e.g. `-s -S` for gdb) |

## Debugging

* **gdb stub**: `WAEASI_QEMU_ARGS="-s -S"` makes QEMU wait for gdb on
  port `1234` before executing any instruction.
* **Interrupt trace**: `WAEASI_QEMU_ARGS="-d int,cpu_reset -D qemu.log"`
  logs every IDT delivery and CPU reset to `qemu.log`.
* **debugcon**: `WAEASI_QEMU_ARGS="-debugcon file:debug.log
  -global isa-debugcon.iobase=0xe9"` exposes I/O port `0xe9` as a
  no-frills debug stream useful before the UART is up.

## Status

The full `cargo krun` pipeline currently builds the kernel, assembles
a UEFI-bootable disk image, and launches Limine v12.3.3 + edk2 OVMF in
QEMU.  Limine successfully discovers the kernel ELF, parses the
request markers, prints its diagnostic banner, and reaches the
hand-off stage.  Booting end-to-end on QEMU 11.0 with the bundled
edk2 firmware is the active integration item — the build / image /
launcher infrastructure itself is complete and reproducible across
Windows, Linux and macOS hosts.

## Layout reference

```
WAeasi/
├── kernel/                       no_std Rust kernel
│   ├── linker.ld                 keeps .requests + .multiboot_header
│   └── src/boot/limine.rs        Limine request markers + parser
│
├── bootloader/limine/            BOOTX64.EFI + LICENSE (BSD-2)
│
├── tools/runner/                 Rust binary invoked by `cargo krun`
│   └── src/{main,disk,qemu,limine}.rs
│
├── .cargo/config.toml            cargo aliases (kbuild / krun / ...)
├── targets/x86_64-waeasi.json    custom Rust target
├── docs/architecture.md          kernel architecture overview
└── docs/booting.md               (this file)
```
