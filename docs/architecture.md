# WAeasi Architecture

## Цели проекта

WAeasi — это **специализированная операционная система** для запуска
WebAssembly-компонентов на голом железе.  Она не претендует на роль
desktop-OS; её ниша — облачный/edge рантайм.

Важно понимать разницу:

| | Linux + Docker | WAeasi |
|--|---|---|
| Размер ядра | ~30M LoC | ~10-30K LoC (цель) |
| Изоляция | namespaces + cgroups + paging | Wasm bytecode (software fault isolation) |
| Context switch | дорогой (TLB flush, регистры) | нулевой — все Wasm в одном AS |
| ABI | POSIX (~400 syscall) | WASI Preview 2 |
| Холодный старт | 100-500 ms | <5 ms (целевой) |

## Слои

```
┌──────────────────────────────────────────────────────────┐
│  Wasm component (untrusted)        — user space          │
├──────────────────────────────────────────────────────────┤
│  WASI Preview 2 host functions     — kernel/wasi/        │
│  (clocks, fs, io, random, sockets)                       │
├──────────────────────────────────────────────────────────┤
│  Wasm engine + async executor      — kernel/{wasm,sched} │
├──────────────────────────────────────────────────────────┤
│  Memory + drivers + net + fs       — kernel/{memory,...} │
├──────────────────────────────────────────────────────────┤
│  Arch HAL (x86_64 / aarch64)       — kernel/arch         │
└──────────────────────────────────────────────────────────┘
```

## Single Address Space (SAS)

Каждая Wasm-инстанция получает собственный диапазон в большом 64 GiB
**виртуальном** регионе (`memory::linear_mem::ARENA_BASE`).  Все диапазоны
лежат в одном PML4, потому что:

* Wasm-инструкции `i32.load/store` сами проверяют bounds (software-MMU);
* Cranelift/Winch генерируют это бесплатно через guard-pages + sigsegv;
* Перезагрузка CR3 не нужна → дешёвые "переключения контекста" в виде
  `await` точек в Rust async.

## Async-first

Ядро не имеет потоков ОС.  Есть один (или один-на-CPU) экземпляр
`sched::executor::Executor`, который крутит `Future`'ы.  Каждый
Wasm-инстанс — это `Future`, чей `poll`:

1. Прокручивает байт-код до следующего host-call'а либо до конца fuel'а.
2. Если host-call блокирующий (например `wasi:io/streams.blocking-read`),
   возвращает `Poll::Pending` и регистрирует waker в `sched::reactor`.
3. Драйвер по IRQ (NIC/timer/block) вызывает `waker.wake()`,
   и инстанция возвращается в очередь.

Ровно одна стек-фрейма на CPU. Никаких аппаратных переключений контекста.

## Граница доверия

| Кому доверяем | Что может |
|---|---|
| Wasm-компонент   | только через WASI capabilities |
| WASI host (kernel) | имеет полные привилегии CPU |
| Драйвер          | имеет полные привилегии CPU, но изолирован модулем |

Поскольку Wasm — verified bytecode, kernel-mode не нужно подтверждать
безопасность памяти отдельно: верификатор уже сделал это.

## Дорожная карта

* **Stage 1 (этот репо)** — каркас: boot, GDT/IDT, APIC, heap, executor,
  WASI-стабы, заглушки virtio.
* **Stage 2** — настоящий wasmi-интерпретатор; virtqueue NIC/blk; TCP/IP
  с реальной reassembly-логикой.
* **Stage 3** — Cranelift JIT (per-instance machine code в read-execute
  страницах); SMP + work-stealing executor.
* **Stage 4** — реализация HTTP-сервера и TLS-терминатора как Wasm-компонентов
  → готовый Cloud-Native рантайм.
