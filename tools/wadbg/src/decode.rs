//! Architectural exception decoders.
//!
//! Maps the raw `ec` integer that the CPU pushes for `#PF`, `#GP`,
//! `#TS`, `#NP`, `#SS`, `#AC` into a list of human-readable bit names
//! plus a one-line summary.  The output is intended to be glanceable:
//!
//!   `ec=0x0a [WRITE | RSVD]  →  write to a present PT entry whose
//!                                bit 63 was set while EFER.NXE=0`
//!
//! The `ExceptionKind` enum mirrors the strings the kernel-side
//! `idt::dump_and_halt` emits, so the parser in `parse.rs` can match
//! by prefix.

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ExceptionKind {
    Pf,        // #PF page fault
    Gp,        // #GP general protection fault
    Ud,        // #UD invalid opcode
    Df,        // #DF double fault
    Ts,        // #TS invalid TSS
    Np,        // #NP segment not present
    Ss,        // #SS stack-segment fault
    Ac,        // #AC alignment check
    Other,     // every other vector — name only, no error-code decode
}

impl ExceptionKind {
    pub fn from_header(h: &str) -> Self {
        // header looks like `=== EXCEPTION #PF  page fault ===`
        let h = h.trim();
        if h.contains("#PF") { Self::Pf }
        else if h.contains("#GP") { Self::Gp }
        else if h.contains("#UD") { Self::Ud }
        else if h.contains("#DF") { Self::Df }
        else if h.contains("#TS") { Self::Ts }
        else if h.contains("#NP") { Self::Np }
        else if h.contains("#SS") { Self::Ss }
        else if h.contains("#AC") { Self::Ac }
        else { Self::Other }
    }
}

#[derive(Debug, Clone)]
pub struct Decoded {
    pub bits:    Vec<&'static str>,
    pub summary: String,
}

pub fn decode(kind: ExceptionKind, ec: Option<u64>, cr2: Option<u64>) -> Decoded {
    let ec = ec.unwrap_or(0);
    match kind {
        ExceptionKind::Pf => decode_pf(ec, cr2),
        ExceptionKind::Gp | ExceptionKind::Np
            | ExceptionKind::Ss | ExceptionKind::Ts => decode_segment_ec(ec),
        ExceptionKind::Ac => Decoded {
            bits: vec!["ALIGN_CHECK"],
            summary: "alignment check (CR0.AM=1, RFLAGS.AC=1; misaligned access)".into(),
        },
        _ => Decoded { bits: Vec::new(), summary: String::new() },
    }
}

fn decode_pf(ec: u64, cr2: Option<u64>) -> Decoded {
    let mut bits = Vec::new();
    if ec & (1 << 0) != 0 { bits.push("PRESENT"); }    else { bits.push("NOT_PRESENT"); }
    if ec & (1 << 1) != 0 { bits.push("WRITE"); }      else { bits.push("READ"); }
    if ec & (1 << 2) != 0 { bits.push("USER"); }       else { bits.push("SUPERVISOR"); }
    if ec & (1 << 3) != 0 { bits.push("RSVD"); }
    if ec & (1 << 4) != 0 { bits.push("FETCH"); }
    if ec & (1 << 5) != 0 { bits.push("PROT_KEY"); }
    if ec & (1 << 6) != 0 { bits.push("SHADOW"); }
    if ec & (1 << 15) != 0 { bits.push("SGX"); }

    let access = if ec & (1 << 4) != 0 {
        "instruction fetch"
    } else if ec & (1 << 1) != 0 {
        "write"
    } else {
        "read"
    };
    let cause = match (ec & 1 != 0, ec & (1 << 3) != 0) {
        (false, _)    => "page is not present",
        (true,  true) => "PT entry has a reserved/NX bit set without EFER.NXE",
        (true,  false) => "permission violation (W^X, RO, ring boundary)",
    };
    let target = match cr2 {
        Some(v) if classify_va(v) == VaClass::Higher => format!(" → kernel VA {v:#018x}"),
        Some(v) if classify_va(v) == VaClass::DirectMap => format!(" → direct-map VA {v:#018x}"),
        Some(v) if classify_va(v) == VaClass::Identity  => format!(" → identity VA {v:#018x}"),
        Some(v) => format!(" → non-canonical/raw VA {v:#018x}"),
        None    => String::new(),
    };
    Decoded {
        bits,
        summary: format!("#PF on {access}: {cause}{target}"),
    }
}

#[derive(PartialEq, Eq)]
enum VaClass { Higher, DirectMap, Identity, Other }

fn classify_va(v: u64) -> VaClass {
    if v >= 0xFFFF_FFFF_8000_0000 { VaClass::Higher }
    else if v >= 0xFFFF_8000_0000_0000 && v < 0xFFFF_FF00_0000_0000 { VaClass::DirectMap }
    else if v < 0x4000_0000 { VaClass::Identity }
    else { VaClass::Other }
}

fn decode_segment_ec(ec: u64) -> Decoded {
    // Selector error code: bits 0..15 are EXT|TBL[2]|INDEX[13].
    let ext  = ec & 0x1 != 0;          // referenced from external event
    let tbl  = (ec >> 1) & 0x3;         // 0=GDT 1=IDT 2=LDT 3=IDT
    let idx  = (ec >> 3) & 0x1FFF;
    let mut bits = Vec::new();
    if ext { bits.push("EXTERNAL"); }
    bits.push(match tbl {
        0     => "GDT",
        1 | 3 => "IDT",
        2     => "LDT",
        _     => "?",
    });
    Decoded {
        bits,
        summary: format!(
            "selector error: table={} index={} ({}; from {}-event)",
            match tbl { 0 => "GDT", 1|3 => "IDT", 2 => "LDT", _ => "?" },
            idx,
            if idx == 0 { "null selector" } else { "non-zero selector" },
            if ext { "external" } else { "internal" },
        ),
    }
}
