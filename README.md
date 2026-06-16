# WAeasi

A bare-metal microkernel for running WebAssembly Component Model
workloads — written in `no_std` Rust. WAeasi is a runtime for cloud
and edge servers; it is not a desktop OS.

## What it is

WAeasi is an **operating system kernel**, but a specialised one. It
boots on physical hardware (or a virtual machine), takes ownership of
CPU, memory, network and disk the way Linux or a BSD does, and exposes
a stable system-call interface to user code. The difference is what
that interface looks like and what kind of user code runs on top.

| | Linux / BSD | WAeasi |
|---|---|---|
| Boots on bare metal | yes | yes |
| User-space ABI | POSIX (~400 syscalls) | WASI Preview 2 |
| What runs on it | ELF binaries, scripts, containers | Wasm Component Model artefacts |
| Process isolation | MMU + ring 0/3 + namespaces | Wasm bytecode (software fault isolation) |
| Drivers, scheduler, TCP/IP | in-kernel | in-kernel |
| Languages in user space | any compiled to ELF | any compiled to Wasm + WASI |

It is **not** a desktop OS, a hypervisor, a container runtime or a
library OS. There is no shell, no init system in the SysV sense, no
GUI layer. What it does provide is an async kernel that schedules Wasm
components as `Future`s in a single shared address space, with all
their syscalls served by an in-kernel WASI implementation.

The closest existing categories are **unikernels** (MirageOS, Nanos,
Unikraft) and **edge runtimes** (the layer Cloudflare Workers and
Fastly Compute@Edge run on top of their host kernel). WAeasi pushes
that idea one step further: there is no host kernel underneath — the
Wasm runtime *is* the kernel.

## What it replaces

A typical cloud workload stack today looks like this:

```
hardware → Linux kernel → container runtime → language runtime → app
```

WAeasi collapses the middle three layers:

```
hardware → WAeasi kernel → Wasm component
```

This removes ~30 million lines of code (Linux + Docker + glibc + most
of a language runtime) from the trusted base, drops cold-start time
from hundreds of milliseconds to single-digit milliseconds, and
replaces POSIX permission models with explicit, signed capability
tokens issued at component load.

## Who it is for

* Operators of edge / serverless platforms who want predictable
  latency and a small attack surface.
* Teams building dense multi-tenant compute where one Linux VM per
  tenant is too expensive and one container per tenant is too leaky.
* People who already write WebAssembly Component Model code and want
  the rest of the stack to disappear.

It is not aimed at general-purpose servers, desktops, or any workload
that depends on the POSIX API surface.

## Design

| Property | Value |
|---|---|
| Kernel ABI | WASI Preview 2 (no POSIX) |
| Isolation | Wasm bytecode (software fault isolation) |
| Address space | Single shared 64-bit virtual range |
| Concurrency | Async executor; every component is a `Future` |
| Memory safety | Rust + `no_std` + no GC |
| Target | x86_64 (aarch64 planned) |

A Wasm component cannot perform any observable action without holding
the matching capability token. Capabilities are issued by the kernel
based on the component's manifest and are enforced at every host call.

## Repository layout

```
WAeasi/
├── kernel/                no_std Rust microkernel
│   └── src/
│       ├── arch/          x86_64 boot, GDT, IDT, APIC
│       ├── memory/        frame allocator, paging, heap, linear memory
│       ├── sched/         async executor, tasks, reactor
│       ├── wasm/          parser, validator, interpreter, JIT, hot reload
│       ├── wasi/          WASI Preview 2 host functions
│       ├── jit/           Cranelift-style code generator
│       ├── http/          HTTP/1.1 + HTTP/2 (HPACK)
│       ├── crypto/        SHA-256/512, Ed25519, X25519, ChaCha20-Poly1305, TLS 1.3
│       ├── net/           Ethernet, IPv4/6, ARP, ICMP, TCP, UDP
│       ├── fs/            VFS, in-memory fs
│       ├── drivers/       virtio-net / virtio-blk, serial, PCI, HPET
│       ├── acpi/          RSDP, XSDT, MADT, MCFG, HPET
│       ├── security/      capabilities, W^X, SMEP/SMAP, Spectre mitigations
│       ├── obs/           tracing, metrics
│       └── ipc/           message channels
│
├── components/            example Wasm components
│   ├── hello-world/
│   ├── healthz/
│   ├── http-server/
│   ├── tls-terminator/
│   └── manifest/          shared TOML schema parser
│
├── wit/                   WIT IDL — single source of truth for ABI
│   └── waeasi/            world.wit, observability.wit, types.wit
│
├── sdk/                   official SDKs
│   ├── typescript/        @waeasi/sdk      (npm)
│   ├── python/            waeasi           (PyPI)
│   └── go/                waeasi.dev/sdk   (Go module)
│
├── builder/               language-agnostic build pipeline (Rust binary)
│
├── tools/
│   ├── runner/            QEMU launcher
│   ├── waeasi-init/       component scaffolder
│   ├── waeasictl/         operator CLI (list/run/inspect/logs/metrics)
│   └── testkit/           kernel test harness
│
├── targets/               x86_64-waeasi.json (custom Rust target)
├── docs/architecture.md
└── Cargo.toml             workspace root
```

## Building

Requires a nightly Rust toolchain (pinned in `rust-toolchain.toml`):

```bash
rustup component add rust-src llvm-tools-preview
cargo kbuild       # build the kernel
cargo krun         # run the kernel under QEMU
```

`cargo kbuild` and `cargo krun` are aliases defined in
`.cargo/config.toml`; both target `x86_64-waeasi` from `targets/`.

## Writing components

Components can be written in any language that compiles to a Wasm
component model artefact targeting WASI Preview 2. The repository
ships official SDKs for three:

```bash
# TypeScript
npm create waeasi@latest hello

# Python
waeasi init hello

# Go (TinyGo + waeasi-builder)
tinygo build -target=wasip2 -o dist/raw.wasm ./cmd/hello
waeasi-builder build --component dist/raw.wasm --manifest waeasi.toml --key env:WAEASI_SIGN_KEY
```

The build pipeline in each SDK produces a signed `.waeasi-bundle`
containing the Wasm component, manifest, and Ed25519 signature. The
bundle is what `waeasictl run` ships to a running kernel.

See [`sdk/README.md`](sdk/README.md) for the full SDK overview and
[`docs/architecture.md`](docs/architecture.md) for the kernel design.

## Status

| Area | State |
|---|---|
| Boot, paging, heap, async executor | implemented |
| WASI Preview 2 host functions | implemented |
| TCP/IP, ARP, ICMP, virtio-net | implemented |
| TLS 1.3 + HTTP/1.1 + HTTP/2 (HPACK) | implemented |
| Ed25519, X25519, ChaCha20-Poly1305, SHA-256/512 | implemented |
| Cranelift-style JIT (interpreter + lowering) | implemented |
| ACPI walk (RSDP → XSDT → MADT/HPET/MCFG) | implemented |
| Hot reload of components | implemented |
| Component Model loader, snapshot mmap, CoW pool | planned |
| AOT precompile cache | planned |
| Engine deduplication | planned |

## License

MIT OR Apache-2.0
