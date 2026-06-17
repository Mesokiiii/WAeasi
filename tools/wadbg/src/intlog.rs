//! Parser for QEMU `-d int` interrupt logs.
//!
//! A single exception line in the QEMU log looks like:
//!
//! ```text
//!      0: v=06 e=0000 i=0 cpl=0 IP=0008:0000000000000003 \
//!         pc=0000000000000003 SP=0010:00000000000f5c78  \
//!         env->regs[R_EAX]=...
//! ```
//!
//! We extract `v`, `e`, `IP=cs:rip`, `pc`, decode `v` to a vector
//! name, and resolve the RIP against the kernel ELF.  Triple-fault
//! markers are reported on their own.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::ExitCode;

use crate::decode::{decode, ExceptionKind};
use crate::pretty::Style;
use crate::resolve::Resolver;

pub fn run(r: &mut Resolver, _st: &Style, p: &Path) -> ExitCode {
    let f = match File::open(p) {
        Ok(f) => f,
        Err(e) => { eprintln!("wadbg: cannot open {}: {e}", p.display()); return ExitCode::from(1); }
    };
    let rdr = BufReader::new(f);
    let mut count = 0;
    for line in rdr.lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed == "Triple fault" {
            println!("⚠  Triple fault — three nested exceptions, CPU reset.");
            count += 1;
            continue;
        }
        if let Some(rec) = parse_exception(trimmed) {
            print_record(r, &rec);
            count += 1;
        }
    }
    if count == 0 {
        println!("(no exception records found in {})", p.display());
    }
    ExitCode::from(0)
}

#[derive(Debug)]
struct Record {
    vec:  u8,
    ec:   u64,
    rip:  u64,
}

fn parse_exception(line: &str) -> Option<Record> {
    // Anchor on the `v=` token so we ignore the noisy header dumps
    // (`CPU Reset`, register dumps).  Format is space-separated
    // key=value pairs.
    let v_pos = line.find("v=")?;
    let after_v = &line[v_pos + 2..];
    let vec_hex = after_v.split_whitespace().next()?;
    let vec     = u8::from_str_radix(vec_hex, 16).ok()?;

    let ec = if let Some(p) = line.find("e=") {
        let s = &line[p + 2..];
        let tok = s.split_whitespace().next().unwrap_or("0");
        u64::from_str_radix(tok, 16).unwrap_or(0)
    } else { 0 };

    let rip = if let Some(p) = line.find("pc=") {
        let s = &line[p + 3..];
        let tok = s.split_whitespace().next().unwrap_or("0");
        u64::from_str_radix(tok, 16).unwrap_or(0)
    } else { 0 };

    Some(Record { vec, ec, rip })
}

fn print_record(r: &mut Resolver, rec: &Record) {
    let kind = match rec.vec {
        0  => ExceptionKind::Other,
        6  => ExceptionKind::Ud,
        8  => ExceptionKind::Df,
        10 => ExceptionKind::Ts,
        11 => ExceptionKind::Np,
        12 => ExceptionKind::Ss,
        13 => ExceptionKind::Gp,
        14 => ExceptionKind::Pf,
        17 => ExceptionKind::Ac,
        _  => ExceptionKind::Other,
    };
    let name = match rec.vec {
        0  => "#DE divide error",       1  => "#DB debug",
        2  => "#NMI",                   3  => "#BP breakpoint",
        4  => "#OF overflow",           5  => "#BR bound range",
        6  => "#UD invalid opcode",     7  => "#NM device n/a",
        8  => "#DF double fault",       10 => "#TS invalid TSS",
        11 => "#NP segment !present",   12 => "#SS stack segment",
        13 => "#GP general protection", 14 => "#PF page fault",
        16 => "#MF x87 fp",             17 => "#AC alignment check",
        19 => "#XM SIMD fp",            20 => "#VE virtualization",
        v  => return println!("vec=0x{v:02x} ec={:#x} ip={:#018x}", rec.ec, rec.rip),
    };
    let dec = decode(kind, Some(rec.ec), None);
    let loc = r.resolve(rec.rip);
    println!(
        "{name:24} ec={:#06x} rip={:#018x}  {}{}",
        rec.ec, rec.rip,
        loc.as_ref()
            .map(|l| format!("{}  ({})", l.function, l.file))
            .unwrap_or_default(),
        if dec.summary.is_empty() { String::new() } else { format!("\n  └─ {}", dec.summary) },
    );
}
