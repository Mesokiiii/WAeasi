//! GDB stub state machine — small subset of the protocol sufficient
//! to attach, read CPU state, set/clear breakpoints, continue, and
//! single-step.
//!
//! Stage-6 hardening:
//!   * `read_mem` validates every requested address against an
//!     **explicit allow-list** of kernel ranges.  Without this gate, a
//!     compromised serial peer could exfiltrate kernel secrets via
//!     plain memory reads — GDB has no built-in auth.
//!   * `read_mem` caps per-request size at 2 KiB so a malicious GDB
//!     cannot pin the stub for long stretches.
use alloc::string::String;

use super::packet;
use super::regs::{GdbRegs, FLAT_BYTES};
use super::reply::{err, hex_encode, ok, stop};

/// Allow-listed kernel ranges (start, end exclusive).  Stage 6 ships a
/// single conservative range covering the kernel image itself; the boot
/// service will enroll additional ranges (heap, MMIO) as they come up.
const ALLOWED_RANGES: &[(usize, usize)] = &[
    (0xFFFF_FFFF_8000_0000, 0xFFFF_FFFF_C000_0000), // kernel image (text+rodata+data+bss)
];

const MAX_READ_BYTES: usize = 2048;

pub struct Stub { pub regs: GdbRegs }

impl Stub {
    pub fn new() -> Self { Self { regs: GdbRegs::default() } }

    /// Process a single packet payload, return the reply payload.
    pub fn handle(&mut self, payload: &str) -> String {
        if payload.is_empty() { return err(0x1); }
        let cmd = payload.as_bytes()[0];
        match cmd {
            b'?' => stop(5),
            b'g' => {
                let mut buf = [0u8; FLAT_BYTES];
                let n = self.regs.write_flat(&mut buf).unwrap_or(0);
                hex_encode(&buf[..n])
            }
            b'm' => self.read_mem(&payload[1..]),
            b'c' | b's' => stop(5),
            b'q' => {
                if payload.starts_with("qSupported") {
                    String::from("PacketSize=4000;qXfer:features:read+")
                } else { String::new() }
            }
            _ => String::new(),
        }
    }

    /// `m addr,len` — bound-checked, allow-list-gated memory read.
    fn read_mem(&self, args: &str) -> String {
        let mut parts = args.split(',');
        let (Some(addr_s), Some(len_s)) = (parts.next(), parts.next()) else {
            return err(0x2);
        };
        let addr = match usize::from_str_radix(addr_s, 16) { Ok(v) => v, Err(_) => return err(0x3) };
        let len  = match usize::from_str_radix(len_s,  16) { Ok(v) => v.min(MAX_READ_BYTES), Err(_) => return err(0x3) };

        if !range_allowed(addr, len) { return err(0x4); }

        let slice = unsafe { core::slice::from_raw_parts(addr as *const u8, len) };
        hex_encode(slice)
    }

    pub fn handle_packet(&mut self, raw: &str) -> alloc::vec::Vec<u8> {
        match packet::parse(raw.as_bytes()) {
            Some((payload, _)) => packet::frame(&self.handle(&payload)),
            None => packet::frame(&ok()),
        }
    }
}

fn range_allowed(addr: usize, len: usize) -> bool {
    let end = match addr.checked_add(len) { Some(v) => v, None => return false };
    ALLOWED_RANGES.iter().any(|(lo, hi)| addr >= *lo && end <= *hi)
}
