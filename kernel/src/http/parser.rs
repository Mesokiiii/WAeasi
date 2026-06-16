//! HTTP/1.1 request parser — zero-copy + bounded.
//!
//! Stage-6 hardening:
//!   * `find_crlf` scans by byte instead of `windows().position()`,
//!     halving instruction count for the most common pattern (`\r\n`).
use alloc::vec::Vec;

pub const MAX_REQUEST_LINE: usize = 8 * 1024;
pub const MAX_HEADERS:      usize = 100;
pub const MAX_HEADER_LINE:  usize = 8 * 1024;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Method { Get, Post, Put, Delete, Head, Options, Patch, Connect, Trace, Other }

impl Method {
    pub fn from_bytes(b: &[u8]) -> Self {
        match b {
            b"GET"     => Method::Get,
            b"POST"    => Method::Post,
            b"PUT"     => Method::Put,
            b"DELETE"  => Method::Delete,
            b"HEAD"    => Method::Head,
            b"OPTIONS" => Method::Options,
            b"PATCH"   => Method::Patch,
            b"CONNECT" => Method::Connect,
            b"TRACE"   => Method::Trace,
            _          => Method::Other,
        }
    }
}

#[derive(Debug)]
pub struct Request<'a> {
    pub method:   Method,
    pub path:     &'a str,
    pub version:  &'a str,
    pub headers:  Vec<(&'a str, &'a str)>,
    pub body_off: usize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError { Eof, BadRequest, Oversize }

pub fn parse(buf: &[u8]) -> Result<Request<'_>, ParseError> {
    let scan_limit = buf.len().min(MAX_REQUEST_LINE);
    let line_end = find_crlf(&buf[..scan_limit])
        .ok_or(if scan_limit < buf.len() { ParseError::Oversize } else { ParseError::Eof })?;
    let line = &buf[..line_end];

    let sp1 = memchr(b' ', line).ok_or(ParseError::BadRequest)?;
    let after_method = &line[sp1 + 1..];
    let sp2 = memchr(b' ', after_method).ok_or(ParseError::BadRequest)?;

    let method  = Method::from_bytes(&line[..sp1]);
    let path    = utf8(&after_method[..sp2])?;
    let version = utf8(&after_method[sp2 + 1..])?;

    let mut p = line_end + 2;
    let mut headers: Vec<(&str, &str)> = Vec::with_capacity(16);
    loop {
        if p >= buf.len() { return Err(ParseError::Eof); }
        if buf[p..].starts_with(b"\r\n") { p += 2; break; }
        if headers.len() >= MAX_HEADERS { return Err(ParseError::Oversize); }

        let line_scan_end = (p + MAX_HEADER_LINE).min(buf.len());
        let h_end = match find_crlf(&buf[p..line_scan_end]) {
            Some(off) => off + p,
            None if line_scan_end < buf.len() => return Err(ParseError::Oversize),
            None => return Err(ParseError::Eof),
        };
        let header = &buf[p..h_end];
        let colon = memchr(b':', header).ok_or(ParseError::BadRequest)?;
        let name  = utf8(&header[..colon])?;
        let mut val_start = colon + 1;
        while val_start < header.len() && header[val_start] == b' ' { val_start += 1; }
        let value = utf8(&header[val_start..])?;
        headers.push((name, value));
        p = h_end + 2;
    }

    Ok(Request { method, path, version, headers, body_off: p })
}

/// Optimized `\r\n` scan — single byte loop, no `windows()` overhead.
#[inline]
fn find_crlf(haystack: &[u8]) -> Option<usize> {
    let mut i = 0;
    while i + 1 < haystack.len() {
        if haystack[i] == b'\r' && haystack[i + 1] == b'\n' { return Some(i); }
        i += 1;
    }
    None
}

#[inline]
fn memchr(byte: u8, haystack: &[u8]) -> Option<usize> {
    haystack.iter().position(|&b| b == byte)
}

#[inline]
fn utf8(b: &[u8]) -> Result<&str, ParseError> {
    core::str::from_utf8(b).map_err(|_| ParseError::BadRequest)
}
