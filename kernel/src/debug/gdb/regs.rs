//! x86_64 register snapshot for GDB `g` / `G` commands.
//!
//! GDB's "x86_64 target description" expects a fixed register order:
//!   rax, rbx, rcx, rdx, rsi, rdi, rbp, rsp,
//!   r8..r15, rip, eflags, cs, ss, ds, es, fs, gs
//!
//! We pack them little-endian into a flat byte buffer.
#[repr(C)]
#[derive(Copy, Clone, Default, Debug)]
pub struct GdbRegs {
    pub rax: u64, pub rbx: u64, pub rcx: u64, pub rdx: u64,
    pub rsi: u64, pub rdi: u64, pub rbp: u64, pub rsp: u64,
    pub r8:  u64, pub r9:  u64, pub r10: u64, pub r11: u64,
    pub r12: u64, pub r13: u64, pub r14: u64, pub r15: u64,
    pub rip: u64,
    pub eflags: u32,
    pub cs: u32, pub ss: u32, pub ds: u32,
    pub es: u32, pub fs: u32, pub gs: u32,
}

pub const FLAT_BYTES: usize = 16 * 8 + 8 + 4 + 6 * 4;

impl GdbRegs {
    /// Write the GDB-canonical layout into `out`.
    pub fn write_flat(&self, out: &mut [u8]) -> Option<usize> {
        if out.len() < FLAT_BYTES { return None; }
        let mut p = 0;
        for &v in &[self.rax, self.rbx, self.rcx, self.rdx,
                    self.rsi, self.rdi, self.rbp, self.rsp,
                    self.r8, self.r9, self.r10, self.r11,
                    self.r12, self.r13, self.r14, self.r15, self.rip] {
            out[p..p + 8].copy_from_slice(&v.to_le_bytes()); p += 8;
        }
        out[p..p + 4].copy_from_slice(&self.eflags.to_le_bytes()); p += 4;
        for &v in &[self.cs, self.ss, self.ds, self.es, self.fs, self.gs] {
            out[p..p + 4].copy_from_slice(&v.to_le_bytes()); p += 4;
        }
        Some(p)
    }
}
