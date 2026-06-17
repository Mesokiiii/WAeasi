//! Stream parser for kernel crash blocks.
//!
//! The kernel emits exception dumps in this exact shape (see
//! `kernel/src/arch/x86_64/idt_dump.rs`):
//!
//! ```text
//!
//!
//! === EXCEPTION #PF  page fault ===
//!   RIP = 0xffffffff8012add1
//!   CS  = 0x0000000000000008
//!   RSP = 0xffffffff80135c60
//!   SS  = 0x0000000000000010
//!   RFL = 0x0000000000010286
//!   ERR = 0x000000000000000a
//!   CR2 = 0xffffff00000000f0
//!   bt:
//!     0xffffffff80123abc
//!     0xffffffff8012def0
//!   CPU halted.
//! ```
//!
//! The parser is a tiny state machine that emits a `Crash` on the
//! terminating `CPU halted.` line.

use crate::decode::ExceptionKind;

#[derive(Debug, Clone)]
pub struct Crash {
    pub header: String,
    pub kind:   ExceptionKind,
    pub rip:    Option<u64>,
    pub cs:     Option<u64>,
    pub rsp:    Option<u64>,
    pub ss:     Option<u64>,
    pub rfl:    Option<u64>,
    pub err:    Option<u64>,
    pub cr2:    Option<u64>,
    pub bt:     Vec<u64>,
}

pub struct Parser {
    cur:   Option<Crash>,
    in_bt: bool,
}

impl Parser {
    pub fn new() -> Self { Self { cur: None, in_bt: false } }

    /// Feed one line of kernel serial output.  Returns `Some(crash)`
    /// when the line completed a crash block, otherwise `None`.
    pub fn feed(&mut self, line: &str) -> Option<Crash> {
        let trimmed = line.trim();
        if trimmed.starts_with("=== EXCEPTION") {
            self.cur = Some(Crash {
                header: trimmed.to_string(),
                kind:   ExceptionKind::from_header(trimmed),
                rip: None, cs: None, rsp: None, ss: None,
                rfl: None, err: None, cr2: None,
                bt:  Vec::new(),
            });
            self.in_bt = false;
            return None;
        }
        let Some(c) = self.cur.as_mut() else { return None };

        if trimmed == "bt:" { self.in_bt = true; return None; }

        if self.in_bt {
            if let Some(addr) = parse_hex(trimmed) {
                c.bt.push(addr);
                return None;
            }
        }

        if let Some((k, v)) = parse_kv(trimmed) {
            match k {
                "RIP" => c.rip = parse_hex(v),
                "CS"  => c.cs  = parse_hex(v),
                "RSP" => c.rsp = parse_hex(v),
                "SS"  => c.ss  = parse_hex(v),
                "RFL" => c.rfl = parse_hex(v),
                "ERR" => c.err = parse_hex(v),
                "CR2" => c.cr2 = parse_hex(v),
                _     => {}
            }
        }

        if trimmed == "CPU halted." {
            self.in_bt = false;
            return self.cur.take();
        }
        None
    }
}

fn parse_kv(line: &str) -> Option<(&str, &str)> {
    let (k, v) = line.split_once('=')?;
    Some((k.trim(), v.trim()))
}

fn parse_hex(s: &str) -> Option<u64> {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    u64::from_str_radix(s, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "
=== EXCEPTION #PF  page fault ===
  RIP = 0xffffffff8012add1
  CS  = 0x0000000000000008
  RSP = 0xffffffff80135c60
  SS  = 0x0000000000000010
  RFL = 0x0000000000010286
  ERR = 0x000000000000000a
  CR2 = 0xffffff00000000f0
  bt:
    0xffffffff80123abc
    0xffffffff8012def0
  CPU halted.
";

    #[test]
    fn parses_full_block() {
        let mut p = Parser::new();
        let mut got: Option<Crash> = None;
        for line in SAMPLE.lines() {
            if let Some(c) = p.feed(line) { got = Some(c); }
        }
        let c = got.expect("crash block emitted");
        assert_eq!(c.kind, ExceptionKind::Pf);
        assert_eq!(c.rip, Some(0xffffffff8012add1));
        assert_eq!(c.err, Some(0x0a));
        assert_eq!(c.cr2, Some(0xffffff00000000f0));
        assert_eq!(c.bt, vec![0xffffffff80123abc, 0xffffffff8012def0]);
    }
}
