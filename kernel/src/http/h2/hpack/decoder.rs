//! HPACK header decoder — zero-copy hot path.
//!
//! Stage-9 perf rewrite:
//!   * `HeaderField<'a>` carries `Cow<'a, str>` instead of `String`.
//!     Static-table indexed → `Cow::Borrowed(&'static str)` (no copy).
//!     Literal-without-indexing non-Huffman → `Cow::Borrowed(&'a str)`
//!     borrowed from the input buffer (no copy).
//!     Dynamic-table indexed and Huffman-decoded → `Cow::Owned(String)`.
//!   * Dynamic-table size tracked per RFC 7541 § 4.1.
//!   * Hard cap (64 KiB) defeats decoder DoS.
use alloc::borrow::Cow;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use super::huffman;
use super::static_table;

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeError { Truncated, BadInteger, BadIndex, BadHuffman, OverSize }

#[derive(Debug, Clone)]
pub struct HeaderField<'a> {
    pub name:  Cow<'a, str>,
    pub value: Cow<'a, str>,
}

impl<'a> HeaderField<'a> {
    /// RFC 7541 § 4.1 entry size formula.
    pub fn size(&self) -> usize { self.name.len() + self.value.len() + 32 }
}

/// Owned variant for the dynamic table (must outlive the input buffer).
#[derive(Debug, Clone)]
struct OwnedHF { name: String, value: String }

const STATIC_LEN: usize = 61;
const HARD_CAP:   usize = 64 * 1024;

pub struct Decoder {
    dynamic:  VecDeque<OwnedHF>,
    pub max_size: usize,
    cur_size: usize,
}

impl Decoder {
    pub fn new(max_size: usize) -> Self {
        Self {
            dynamic:  VecDeque::new(),
            max_size: max_size.min(HARD_CAP),
            cur_size: 0,
        }
    }

    pub fn decode<'a>(&mut self, mut buf: &'a [u8]) -> Result<Vec<HeaderField<'a>>, DecodeError> {
        let mut out = Vec::with_capacity(8);
        while !buf.is_empty() {
            let b = buf[0];
            if b & 0x80 != 0 {
                let (idx, rest) = read_int(buf, 7)?;
                buf = rest;
                out.push(self.lookup(idx)?);
            } else if b & 0x40 != 0 {
                let (idx, rest) = read_int(buf, 6)?;
                buf = rest;
                let (name, b1) = self.read_name(buf, idx)?;
                let (value, b2) = read_string(b1)?;
                buf = b2;

                // Push to dynamic — owned.
                let owned = OwnedHF {
                    name:  name.clone().into_owned(),
                    value: value.clone().into_owned(),
                };
                self.push_dynamic(owned)?;
                out.push(HeaderField { name, value });
            } else if b & 0x20 != 0 {
                let (sz, rest) = read_int(buf, 5)?;
                buf = rest;
                if sz > HARD_CAP { return Err(DecodeError::OverSize); }
                self.max_size = sz;
                self.evict();
            } else {
                let (idx, rest) = read_int(buf, 4)?;
                buf = rest;
                let (name, b1) = self.read_name(buf, idx)?;
                let (value, b2) = read_string(b1)?;
                buf = b2;
                out.push(HeaderField { name, value });
            }
        }
        Ok(out)
    }

    fn push_dynamic(&mut self, hf: OwnedHF) -> Result<(), DecodeError> {
        let sz = hf.name.len() + hf.value.len() + 32;
        if sz > self.max_size {
            self.dynamic.clear(); self.cur_size = 0;
            return Ok(());
        }
        self.dynamic.push_front(hf);
        self.cur_size += sz;
        self.evict();
        Ok(())
    }

    fn evict(&mut self) {
        while self.cur_size > self.max_size {
            match self.dynamic.pop_back() {
                Some(e) => self.cur_size = self.cur_size
                    .saturating_sub(e.name.len() + e.value.len() + 32),
                None => { self.cur_size = 0; break; }
            }
        }
    }

    /// Resolve `idx` into a `HeaderField` — `'static` for the static
    /// table, owned clone for the dynamic table.
    fn lookup<'a>(&self, idx: usize) -> Result<HeaderField<'a>, DecodeError> {
        if (1..=STATIC_LEN).contains(&idx) {
            let (n, v) = static_table::get(idx).ok_or(DecodeError::BadIndex)?;
            return Ok(HeaderField {
                name:  Cow::Borrowed(n),
                value: Cow::Borrowed(v),
            });
        }
        let dyn_idx = idx.checked_sub(STATIC_LEN + 1).ok_or(DecodeError::BadIndex)?;
        let e = self.dynamic.get(dyn_idx).ok_or(DecodeError::BadIndex)?;
        Ok(HeaderField {
            name:  Cow::Owned(e.name.clone()),
            value: Cow::Owned(e.value.clone()),
        })
    }

    fn read_name<'a>(&self, buf: &'a [u8], idx: usize)
        -> Result<(Cow<'a, str>, &'a [u8]), DecodeError>
    {
        if idx == 0 {
            let (n, rest) = read_string(buf)?;
            Ok((n, rest))
        } else {
            Ok((self.lookup(idx)?.name.into_owned().into(), buf))
        }
    }
}

fn read_int(buf: &[u8], prefix_bits: u8) -> Result<(usize, &[u8]), DecodeError> {
    if buf.is_empty() { return Err(DecodeError::Truncated); }
    let mask = (1u8 << prefix_bits) - 1;
    let mut value = (buf[0] & mask) as usize;
    if value < mask as usize { return Ok((value, &buf[1..])); }
    let mut p = 1; let mut m = 0;
    loop {
        if p >= buf.len() { return Err(DecodeError::Truncated); }
        let b = buf[p]; p += 1;
        value += ((b & 0x7F) as usize) << m;
        if b & 0x80 == 0 { return Ok((value, &buf[p..])); }
        m += 7;
        if m > 28 { return Err(DecodeError::BadInteger); }
    }
}

/// Read a header value/name string.  If the wire form is **not**
/// Huffman-encoded, we borrow directly from the input — zero copy.
fn read_string(buf: &[u8]) -> Result<(Cow<'_, str>, &[u8]), DecodeError> {
    if buf.is_empty() { return Err(DecodeError::Truncated); }
    let huff = buf[0] & 0x80 != 0;
    let (len, rest) = read_int(buf, 7)?;
    if rest.len() < len { return Err(DecodeError::Truncated); }
    let bytes = &rest[..len];
    let after = &rest[len..];

    if huff {
        let raw = huffman::decode(bytes).map_err(|_| DecodeError::BadHuffman)?;
        let s = String::from_utf8(raw).map_err(|_| DecodeError::BadHuffman)?;
        Ok((Cow::Owned(s), after))
    } else {
        // Zero-copy: validate UTF-8 in place; if valid, borrow.
        match core::str::from_utf8(bytes) {
            Ok(s)  => Ok((Cow::Borrowed(s), after)),
            Err(_) => Err(DecodeError::BadHuffman),
        }
    }
}
