//! Pretty-printing layer with optional ANSI colour.
//!
//! Output style:
//!
//! ```text
//! ╭─ wadbg ─────────────────────────────────────────────────────────
//! │ EXCEPTION  #PF  page fault
//! │ ec=0x0a [WRITE | RSVD]
//! │ #PF on write: PT entry has a reserved/NX bit set without EFER.NXE
//! │ RIP    0xffffffff8012add1   memory::heap::init at heap.rs:42:5
//! │ CR2    0xffffff00000000f0   (mmio arena, vector 0)
//! ╰─
//! ```

use crate::decode::decode;
use crate::parse::Crash;
use crate::resolve::{Location, Resolver};

#[derive(Copy, Clone)]
pub enum Color { Auto, Off }

pub struct Style {
    use_color: bool,
}

impl Style {
    pub fn for_stream<S>(c: Color, _s: S) -> Self {
        let use_color = match c {
            Color::Off  => false,
            // Conservative: only enable colour when both stdout and
            // stderr are likely terminals.  We don't pull `is-terminal`
            // as a dep — assume "no" on Windows shells unless the
            // env var WADBG_COLOR=1 forces it.
            Color::Auto => std::env::var_os("WADBG_COLOR").is_some(),
        };
        Self { use_color }
    }

    fn dim(&self,  s: &str) -> String { if self.use_color { format!("\x1b[2m{s}\x1b[0m")  } else { s.into() } }
    #[allow(dead_code)]
    fn red(&self,  s: &str) -> String { if self.use_color { format!("\x1b[31m{s}\x1b[0m") } else { s.into() } }
    fn cyan(&self, s: &str) -> String { if self.use_color { format!("\x1b[36m{s}\x1b[0m") } else { s.into() } }
    fn bold(&self, s: &str) -> String { if self.use_color { format!("\x1b[1m{s}\x1b[0m")  } else { s.into() } }

    pub fn print_resolved(&self, addr: u64, loc: &Location) {
        println!(
            "{} {:#018x}  {}  {}",
            self.cyan("addr"), addr,
            self.bold(&loc.function),
            self.dim(&loc.file),
        );
    }
    pub fn print_unresolved(&self, addr: u64) {
        println!("{} {:#018x}  {}", self.cyan("addr"), addr, self.dim("(unresolved)"));
    }
}

/// Render a finished crash block, optionally enriched with source
/// locations resolved via `r`.
pub fn format_crash(c: &Crash, r: &mut Resolver) -> String {
    let st = Style { use_color: false };  // colour layer is for the streaming print path; this path is plain text so logs stay grep-friendly
    let dec = decode(c.kind, c.err, c.cr2);

    let bits = if dec.bits.is_empty() {
        String::new()
    } else {
        format!(" [{}]", dec.bits.join(" | "))
    };

    let rip_loc  = c.rip.and_then(|a| r.resolve(a));
    let cr2_loc  = c.cr2.and_then(|a| r.resolve(a));

    let mut out = String::new();
    out.push_str("+-- wadbg ---------------------------------------------------------\n");
    out.push_str(&format!("| {}\n", st.bold(c.header.trim_start_matches("=== ").trim_end_matches(" ==="))));
    if let Some(ec) = c.err {
        out.push_str(&format!("| ec={:#x}{}\n", ec, bits));
    }
    if !dec.summary.is_empty() {
        out.push_str(&format!("| {}\n", dec.summary));
    }
    out.push_str(&format!("| RIP   {:#018x}{}\n", c.rip.unwrap_or(0), fmt_loc(&rip_loc)));
    if let Some(cr2) = c.cr2 {
        out.push_str(&format!("| CR2   {:#018x}{}\n", cr2, fmt_loc(&cr2_loc)));
    }
    out.push_str(&format!(
        "| stack RSP={:#018x} RFL={:#018x}\n",
        c.rsp.unwrap_or(0), c.rfl.unwrap_or(0),
    ));
    if !c.bt.is_empty() {
        out.push_str("| backtrace:\n");
        for (i, addr) in c.bt.iter().enumerate() {
            let loc = r.resolve(*addr);
            out.push_str(&format!(
                "|   #{i:<2} {:#018x}{}\n",
                addr, fmt_loc(&loc),
            ));
        }
    }
    out.push_str("+--\n");
    out
}

fn fmt_loc(loc: &Option<Location>) -> String {
    match loc {
        Some(l) if !l.function.is_empty() && !l.file.is_empty()
            => format!("   {}  ({})", l.function, l.file),
        Some(l) if !l.function.is_empty()
            => format!("   {}", l.function),
        _ => String::new(),
    }
}
