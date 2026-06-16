//! TCP segment header + flag bits.
//!
//! 20-byte fixed header, optional 0-40 bytes of options.
use bitflags::bitflags;

#[repr(C, packed)]
#[derive(Copy, Clone, Debug)]
pub struct Header {
    pub src_port:     u16,
    pub dst_port:     u16,
    pub seq:          u32,
    pub ack:          u32,
    pub offset_flags: u16,    // 4-bit data offset + 6 reserved + 6 flags
    pub window:       u16,
    pub checksum:     u16,
    pub urgent:       u16,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u8 {
        const FIN = 1 << 0;
        const SYN = 1 << 1;
        const RST = 1 << 2;
        const PSH = 1 << 3;
        const ACK = 1 << 4;
        const URG = 1 << 5;
        const ECE = 1 << 6;
        const CWR = 1 << 7;
    }
}

impl Header {
    /// Header length in 32-bit words.
    pub fn data_offset(&self) -> u8 {
        let raw = u16::from_be(self.offset_flags);
        ((raw >> 12) & 0x0F) as u8
    }

    pub fn flags(&self) -> Flags {
        let raw = u16::from_be(self.offset_flags);
        Flags::from_bits_truncate((raw & 0xFF) as u8)
    }

    pub fn set_offset_flags(&mut self, words: u8, flags: Flags) {
        let v = ((words as u16 & 0x0F) << 12) | (flags.bits() as u16);
        self.offset_flags = v.to_be();
    }

    pub fn seq_host(&self) -> u32 { u32::from_be(self.seq) }
    pub fn ack_host(&self) -> u32 { u32::from_be(self.ack) }
}

#[derive(Debug)]
pub struct Segment<'a> {
    pub header:  Header,
    pub options: &'a [u8],
    pub payload: &'a [u8],
}

pub fn parse(buf: &[u8]) -> Option<Segment<'_>> {
    if buf.len() < 20 { return None; }
    let header_ptr = buf.as_ptr() as *const Header;
    let header = unsafe { core::ptr::read_unaligned(header_ptr) };
    let offset = header.data_offset() as usize * 4;
    if offset < 20 || offset > buf.len() { return None; }
    Some(Segment {
        header,
        options: &buf[20..offset],
        payload: &buf[offset..],
    })
}
