//! DER (Distinguished Encoding Rules) reader.
//!
//! Each TLV is `[tag | length | value]`:
//!   * Tag      — 1 byte (multi-byte tags are rare in X.509; we reject).
//!   * Length   — short form (1 byte if MSB clear) or long form
//!     (`0x80 | n`, then `n` BE bytes of length).
//!   * Value    — exactly `length` bytes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Tag(pub u8);

impl Tag {
    pub const BOOLEAN:           Self = Self(0x01);
    pub const INTEGER:           Self = Self(0x02);
    pub const BIT_STRING:        Self = Self(0x03);
    pub const OCTET_STRING:      Self = Self(0x04);
    pub const NULL:              Self = Self(0x05);
    pub const OID:               Self = Self(0x06);
    pub const UTF8_STRING:       Self = Self(0x0C);
    pub const PRINTABLE_STRING:  Self = Self(0x13);
    pub const UTC_TIME:          Self = Self(0x17);
    pub const GENERALIZED_TIME:  Self = Self(0x18);
    pub const SEQUENCE:          Self = Self(0x30);
    pub const SET:               Self = Self(0x31);
    pub fn context_constructed(n: u8) -> Self { Self(0xA0 | n) }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DerError { UnexpectedTag, BadLength, Truncated, MultiByteTag }

pub struct Reader<'a> { buf: &'a [u8], pos: usize }

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self { Self { buf, pos: 0 } }
    pub fn pos(&self) -> usize { self.pos }
    pub fn remaining(&self) -> usize { self.buf.len() - self.pos }
    pub fn eof(&self) -> bool { self.pos >= self.buf.len() }

    /// Read one TLV.  Returns `(tag, value_slice)`.
    pub fn read_tlv(&mut self) -> Result<(Tag, &'a [u8]), DerError> {
        let tag_byte = self.byte()?;
        if tag_byte & 0x1F == 0x1F { return Err(DerError::MultiByteTag); }
        let len = self.read_length()?;
        if self.pos + len > self.buf.len() { return Err(DerError::Truncated); }
        let value = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok((Tag(tag_byte), value))
    }

    /// Read TLV expecting a specific tag.
    pub fn expect(&mut self, tag: Tag) -> Result<&'a [u8], DerError> {
        let (t, v) = self.read_tlv()?;
        if t != tag { return Err(DerError::UnexpectedTag); }
        Ok(v)
    }

    /// Open a SEQUENCE — returns a sub-reader over its body.
    pub fn open_seq(&mut self) -> Result<Reader<'a>, DerError> {
        let body = self.expect(Tag::SEQUENCE)?;
        Ok(Reader::new(body))
    }

    fn byte(&mut self) -> Result<u8, DerError> {
        let b = *self.buf.get(self.pos).ok_or(DerError::Truncated)?;
        self.pos += 1;
        Ok(b)
    }

    fn read_length(&mut self) -> Result<usize, DerError> {
        let first = self.byte()?;
        if first & 0x80 == 0 { return Ok(first as usize); }
        let n = (first & 0x7F) as usize;
        if n == 0 || n > 8 { return Err(DerError::BadLength); }
        let mut len: usize = 0;
        for _ in 0..n {
            len = len.checked_shl(8).ok_or(DerError::BadLength)?
                | self.byte()? as usize;
        }
        Ok(len)
    }
}
