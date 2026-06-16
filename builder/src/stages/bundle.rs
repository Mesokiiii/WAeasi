//! bundle stage — pack everything into the canonical `.waeasi-bundle`.
//!
//! Layout (must match the TS / Python / kernel implementations):
//!
//!   0:  "WAEASIBND" magic           (9 bytes)
//!   9:  u8 version (=1)
//!  10:  u32 LE entry count
//!  14:  for each entry:
//!         u8 name_len  (≤ 64)
//!         bytes name
//!         u64 LE offset
//!         u64 LE length
//!  ...  payload sections, each 8-byte aligned

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::digest;
use crate::error::{BuildError, Result};
use crate::fs;

const MAGIC: &[u8; 9] = b"WAEASIBND";
const VERSION: u8 = 1;
const ALIGN: usize = 8;
const MAX_ENTRIES: usize = 64;
const MAX_NAME: usize = 64;

#[derive(Debug, Clone)]
pub struct BundleEntry<'a> {
    pub name: &'a str,
    pub path: &'a Path,
}

#[derive(Debug)]
pub struct Output {
    pub path:        PathBuf,
    pub size_bytes:  u64,
    pub digest_hex:  String,
    pub entry_count: usize,
}

pub fn assemble(entries: &[BundleEntry<'_>], out_path: &Path) -> Result<Output> {
    if entries.is_empty() {
        return Err(BuildError::Bundle("no entries to bundle".into()));
    }
    if entries.len() > MAX_ENTRIES {
        return Err(BuildError::Bundle(format!(
            "too many entries ({} > {})", entries.len(), MAX_ENTRIES,
        )));
    }
    for e in entries {
        validate_name(e.name)?;
    }

    let mut payloads: Vec<(BundleEntry<'_>, Vec<u8>)> =
        Vec::with_capacity(entries.len());
    for e in entries {
        let data = fs::read(e.path)?;
        payloads.push((e.clone(), data));
    }

    let mut header_size = MAGIC.len() + 1 + 4;
    for (e, _) in &payloads {
        header_size += 1 + e.name.len() + 8 + 8;
    }

    let mut placed: Vec<(BundleEntry<'_>, Vec<u8>, usize)> =
        Vec::with_capacity(payloads.len());
    let mut cursor = pad(header_size, ALIGN);
    for (e, data) in payloads {
        cursor = pad(cursor, ALIGN);
        let off = cursor;
        cursor += data.len();
        placed.push((e, data, off));
    }
    let total = cursor;

    let mut out = vec![0u8; total];
    let mut h = 0;
    out[h..h + MAGIC.len()].copy_from_slice(MAGIC); h += MAGIC.len();
    out[h] = VERSION; h += 1;
    out[h..h + 4].copy_from_slice(&(placed.len() as u32).to_le_bytes()); h += 4;
    for (e, data, off) in &placed {
        out[h] = e.name.len() as u8; h += 1;
        out[h..h + e.name.len()].copy_from_slice(e.name.as_bytes());
        h += e.name.len();
        out[h..h + 8].copy_from_slice(&(*off as u64).to_le_bytes());  h += 8;
        out[h..h + 8].copy_from_slice(&(data.len() as u64).to_le_bytes()); h += 8;
    }
    for (_, data, off) in &placed {
        out[*off..*off + data.len()].copy_from_slice(data);
    }

    fs::write_atomic(out_path, &out)?;

    let mut sha = Sha256::new();
    sha.update(&out);
    let digest_hex = digest::hex_lower(&sha.finalize());

    Ok(Output {
        path:        out_path.into(),
        size_bytes:  out.len() as u64,
        digest_hex,
        entry_count: placed.len(),
    })
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_NAME {
        return Err(BuildError::Bundle(format!("bad entry name length: {name:?}")));
    }
    for b in name.bytes() {
        let ok = b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-');
        if !ok {
            return Err(BuildError::Bundle(format!("unsafe entry name char: {name:?}")));
        }
    }
    Ok(())
}

fn pad(n: usize, align: usize) -> usize {
    let m = n % align;
    if m == 0 { n } else { n + (align - m) }
}
