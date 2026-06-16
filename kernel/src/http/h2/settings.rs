//! SETTINGS frame body — list of (id, u32) pairs.
use alloc::vec::Vec;

#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SettingId {
    HeaderTableSize      = 0x1,
    EnablePush           = 0x2,
    MaxConcurrentStreams = 0x3,
    InitialWindowSize    = 0x4,
    MaxFrameSize         = 0x5,
    MaxHeaderListSize    = 0x6,
    Other                = 0x0,
}

impl SettingId {
    pub fn from_u16(b: u16) -> Self {
        match b {
            0x1 => Self::HeaderTableSize,      0x2 => Self::EnablePush,
            0x3 => Self::MaxConcurrentStreams, 0x4 => Self::InitialWindowSize,
            0x5 => Self::MaxFrameSize,         0x6 => Self::MaxHeaderListSize,
            _   => Self::Other,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings { pub items: Vec<(SettingId, u32)> }

pub fn parse(buf: &[u8]) -> Option<Settings> {
    if buf.len() % 6 != 0 { return None; }
    let mut items = Vec::with_capacity(buf.len() / 6);
    let mut i = 0;
    while i + 6 <= buf.len() {
        let id  = u16::from_be_bytes([buf[i],   buf[i+1]]);
        let val = u32::from_be_bytes([buf[i+2], buf[i+3], buf[i+4], buf[i+5]]);
        items.push((SettingId::from_u16(id), val));
        i += 6;
    }
    Some(Settings { items })
}

pub fn write(out: &mut Vec<u8>, s: &Settings) {
    for (id, v) in &s.items {
        out.extend_from_slice(&(*id as u16).to_be_bytes());
        out.extend_from_slice(&v.to_be_bytes());
    }
}

/// Server preferred default settings — applied at connection start
/// before peer SETTINGS arrives.
pub const SERVER_DEFAULT: &[(SettingId, u32)] = &[
    (SettingId::MaxConcurrentStreams, 256),
    (SettingId::InitialWindowSize,    65_535),
    (SettingId::MaxFrameSize,         16_384),
    (SettingId::MaxHeaderListSize,    16_384),
    (SettingId::EnablePush,           0),
];
