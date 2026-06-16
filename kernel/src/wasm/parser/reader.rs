//! Cursor-style reader over an `&[u8]`.  Bound-checked, slice-returning,
//! and integrates LEB128 helpers via methods.
use super::leb128;
use super::ParseError;

pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self { Self { buf, pos: 0 } }

    pub fn pos(&self) -> usize { self.pos }
    pub fn remaining(&self) -> usize { self.buf.len() - self.pos }
    pub fn eof(&self) -> bool { self.pos >= self.buf.len() }

    pub fn u8(&mut self) -> Result<u8, ParseError> {
        let b = *self.buf.get(self.pos).ok_or(ParseError::UnexpectedEof)?;
        self.pos += 1;
        Ok(b)
    }

    pub fn u32_le(&mut self) -> Result<u32, ParseError> {
        let s = self.bytes(4)?;
        Ok(u32::from_le_bytes(s.try_into().unwrap()))
    }

    pub fn u32_leb(&mut self) -> Result<u32, ParseError> {
        leb128::read_u32(self.buf, &mut self.pos)
    }
    pub fn i32_leb(&mut self) -> Result<i32, ParseError> {
        leb128::read_i32(self.buf, &mut self.pos)
    }
    pub fn i64_leb(&mut self) -> Result<i64, ParseError> {
        leb128::read_i64(self.buf, &mut self.pos)
    }

    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        if self.pos + n > self.buf.len() { return Err(ParseError::UnexpectedEof); }
        let s = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    pub fn name(&mut self) -> Result<&'a str, ParseError> {
        let len = self.u32_leb()? as usize;
        let raw = self.bytes(len)?;
        core::str::from_utf8(raw).map_err(|_| ParseError::Truncated)
    }

    /// Run `f` with a sub-reader covering exactly `len` bytes; ensures
    /// the inner parser doesn't overshoot the section boundary.
    pub fn limited<R, F>(&mut self, len: usize, f: F) -> Result<R, ParseError>
    where F: FnOnce(&mut Reader<'a>) -> Result<R, ParseError>
    {
        let body = self.bytes(len)?;
        let mut sub = Reader::new(body);
        let r = f(&mut sub)?;
        if !sub.eof() { return Err(ParseError::Truncated); }
        Ok(r)
    }
}
