//! `Transfer-Encoding: chunked` (RFC 9112 § 7.1).
//!
//! Body framing:
//! ```text
//!   1*HEXDIG CRLF data CRLF      (one chunk)
//!   ...
//!   "0" CRLF                     (last-chunk)
//!   *trailer CRLF
//!   CRLF
//! ```
use alloc::vec::Vec;

/// Encode a single chunk: `"<hex_len>\r\n<data>\r\n"`.
pub fn encode_chunk(out: &mut Vec<u8>, data: &[u8]) {
    push_hex(out, data.len());
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(data);
    out.extend_from_slice(b"\r\n");
}

/// Append the trailing `0\r\n\r\n` that closes a chunked body.
pub fn finish(out: &mut Vec<u8>) {
    out.extend_from_slice(b"0\r\n\r\n");
}

fn push_hex(out: &mut Vec<u8>, mut n: usize) {
    if n == 0 { out.push(b'0'); return; }
    let mut buf = [0u8; 16];
    let mut i = 0;
    while n > 0 {
        let nibble = (n & 0xF) as u8;
        buf[i] = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
        n >>= 4;
        i += 1;
    }
    for &b in buf[..i].iter().rev() { out.push(b); }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecodeStep<'a> {
    Chunk(&'a [u8]),
    Done,
    NeedMore,
}

/// Single-step decoder.  Returns the slice for the next chunk and how
/// many bytes of `buf` were consumed; `Done` when the terminator was
/// seen; `NeedMore` if `buf` is incomplete.
pub fn decode_step(buf: &[u8]) -> (DecodeStep<'_>, usize) {
    if let Some(eol) = find(buf, b"\r\n") {
        let len_str = &buf[..eol];
        let len = match parse_hex(len_str) { Some(v) => v, None => return (DecodeStep::Done, 0) };
        if len == 0 {
            // last-chunk + final CRLF.
            let total = eol + 2 + 2;
            if buf.len() < total { return (DecodeStep::NeedMore, 0); }
            return (DecodeStep::Done, total);
        }
        let chunk_start = eol + 2;
        let chunk_end = chunk_start + len;
        if buf.len() < chunk_end + 2 { return (DecodeStep::NeedMore, 0); }
        return (DecodeStep::Chunk(&buf[chunk_start..chunk_end]), chunk_end + 2);
    }
    (DecodeStep::NeedMore, 0)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn parse_hex(s: &[u8]) -> Option<usize> {
    let mut v: usize = 0;
    for &b in s {
        let d = match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => return None,
        };
        v = v.checked_mul(16)?.checked_add(d as usize)?;
    }
    Some(v)
}
