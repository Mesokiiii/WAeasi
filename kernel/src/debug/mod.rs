//! In-kernel debug surfaces.
//!
//! Stage 6 ships a **GDB Remote Serial Protocol** stub
//! (`gdb_remote_serial_protocol`) so a developer can connect with:
//!
//! ```bash
//!   gdb path/to/waeasi
//!   (gdb) target remote /dev/ttyS0    # or QEMU's -serial pipe
//! ```
//!
//! and inspect register state, single-step, set breakpoints.  Stage 7
//! adds memory-write / hardware breakpoints.
pub mod gdb;
