//! Output formatting — table, wide, JSON (with proper escaping), YAML.
//!
//! Single source of truth lives in `Format`; every command renders
//! its data once and the formatter picks the right wire format.
use std::sync::atomic::{AtomicU8, Ordering};

use crate::color;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Format { Table = 0, Wide = 1, Json = 2, Yaml = 3 }

static FORMAT: AtomicU8 = AtomicU8::new(Format::Table as u8);

pub fn set(f: Format)     { FORMAT.store(f as u8, Ordering::Relaxed); }
pub fn current() -> Format {
    match FORMAT.load(Ordering::Relaxed) {
        x if x == Format::Wide as u8 => Format::Wide,
        x if x == Format::Json as u8 => Format::Json,
        x if x == Format::Yaml as u8 => Format::Yaml,
        _ => Format::Table,
    }
}

/// Render `headers` × `rows` according to the active format.
/// `status_col` (if set) gets `color::status()` applied in table modes.
pub fn render(headers: &[&str], rows: &[Vec<String>], status_col: Option<usize>) {
    match current() {
        Format::Table => print_table(headers, rows, status_col, false),
        Format::Wide  => print_table(headers, rows, status_col, true),
        Format::Json  => print_json(headers, rows),
        Format::Yaml  => print_yaml(headers, rows),
    }
}

fn print_table(headers: &[&str], rows: &[Vec<String>], status_col: Option<usize>, _wide: bool) {
    if rows.is_empty() && current() != Format::Wide {
        eprintln!("(no items)");
        return;
    }
    // Single-pass widths.
    let mut w: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for r in rows {
        for (i, cell) in r.iter().enumerate() {
            if i >= w.len() { break; }
            if cell.len() > w[i] { w[i] = cell.len(); }
        }
    }
    // Header row (bold).
    let mut hdr = String::new();
    for (i, h) in headers.iter().enumerate() {
        let pad = w.get(i).copied().unwrap_or(h.len());
        let cell = format!("{:width$}", h, width = pad);
        hdr.push_str(&color::paint(color::Color::Bold, &cell));
        if i + 1 < headers.len() { hdr.push_str("  "); }
    }
    println!("{}", hdr);
    // Data rows.
    for r in rows {
        let mut line = String::new();
        for (i, cell) in r.iter().enumerate() {
            let pad = w.get(i).copied().unwrap_or(cell.len());
            let formatted = format!("{:width$}", cell, width = pad);
            if status_col == Some(i) {
                line.push_str(&color::status(&formatted));
            } else {
                line.push_str(&formatted);
            }
            if i + 1 < r.len() { line.push_str("  "); }
        }
        println!("{}", line);
    }
}

fn print_json(headers: &[&str], rows: &[Vec<String>]) {
    let mut out = String::with_capacity(rows.len() * 64);
    out.push('[');
    for (i, r) in rows.iter().enumerate() {
        if i > 0 { out.push(','); }
        out.push('{');
        for (j, h) in headers.iter().enumerate() {
            if j > 0 { out.push(','); }
            json_str(&mut out, h);
            out.push(':');
            json_str(&mut out, r.get(j).map(|s| s.as_str()).unwrap_or(""));
        }
        out.push('}');
    }
    out.push(']');
    println!("{}", out);
}

fn print_yaml(headers: &[&str], rows: &[Vec<String>]) {
    for r in rows {
        println!("- ");
        for (j, h) in headers.iter().enumerate() {
            let v = r.get(j).map(|s| s.as_str()).unwrap_or("");
            println!("  {}: {}", h, yaml_value(v));
        }
    }
}

/// RFC-8259-correct JSON string serializer.  Escapes `"`, `\`, control
/// bytes (`\u00XX`), and ensures the output is always valid JSON even
/// for adversarial server data.
fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => { out.push_str(&format!("\\u{:04x}", c as u32)); }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn yaml_value(s: &str) -> String {
    if s.is_empty() || s.contains(':') || s.contains('#') || s.contains('\n')
       || s.starts_with(' ') || s.ends_with(' ')
    {
        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{}\"", escaped)
    } else { s.to_string() }
}

/// Convenience for legacy `--json` callers.
pub fn set_json(on: bool) { set(if on { Format::Json } else { Format::Table }); }
pub fn is_json()   -> bool { matches!(current(), Format::Json) }

/// Compatibility: `output::table(headers, rows)` w/o status column.
pub fn table(headers: &[&str], rows: &[Vec<String>]) {
    render(headers, rows, None)
}
