//! TLS 1.3 handshake messages (RFC 8446 § 4).
//!
//! Stage-10 expansion: real **ClientHello parser** that extracts the
//! pieces the server needs to drive the handshake — random, session
//! ID, cipher-suite list, extensions (supported_versions + key_share).
use super::TlsError;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HandshakeType {
    ClientHello = 1, ServerHello = 2,
    NewSessionTicket = 4, EndOfEarlyData = 5,
    EncryptedExtensions = 8, Certificate = 11,
    CertificateVerify = 15, Finished = 20, KeyUpdate = 24,
}

impl HandshakeType {
    pub fn from_u8(b: u8) -> Result<Self, TlsError> {
        Ok(match b {
            1=>Self::ClientHello, 2=>Self::ServerHello, 4=>Self::NewSessionTicket,
            5=>Self::EndOfEarlyData, 8=>Self::EncryptedExtensions,
            11=>Self::Certificate, 15=>Self::CertificateVerify,
            20=>Self::Finished, 24=>Self::KeyUpdate,
            _ => return Err(TlsError::DecodeError),
        })
    }
}

pub const HEADER_LEN: usize = 4;

#[derive(Debug)]
pub struct Header { pub kind: HandshakeType, pub len: usize }

pub fn parse_header(buf: &[u8]) -> Result<Header, TlsError> {
    if buf.len() < HEADER_LEN { return Err(TlsError::DecodeError); }
    let kind = HandshakeType::from_u8(buf[0])?;
    let len = ((buf[1] as usize) << 16) | ((buf[2] as usize) << 8) | (buf[3] as usize);
    Ok(Header { kind, len })
}

pub fn frame_into(out: &mut [u8], kind: HandshakeType, body: &[u8]) -> Result<usize, TlsError> {
    let total = HEADER_LEN + body.len();
    if out.len() < total { return Err(TlsError::DecodeError); }
    out[0] = kind as u8;
    out[1] = (body.len() >> 16) as u8;
    out[2] = (body.len() >>  8) as u8;
    out[3] = (body.len() & 0xFF) as u8;
    out[HEADER_LEN..total].copy_from_slice(body);
    Ok(total)
}

/// Parsed ClientHello (server-side projection — fields the server
/// actually consumes per RFC 8446).
#[derive(Debug)]
pub struct ClientHello<'a> {
    pub random:           [u8; 32],
    pub legacy_session_id:&'a [u8],
    pub cipher_suites:    &'a [u8],
    pub key_share_x25519: Option<[u8; 32]>,
    pub supports_tls13:   bool,
}

const EXT_SUPPORTED_VERSIONS: u16 = 0x002B;
const EXT_KEY_SHARE:          u16 = 0x0033;
const GROUP_X25519:           u16 = 0x001D;

pub fn parse_client_hello(buf: &[u8]) -> Result<ClientHello<'_>, TlsError> {
    let mut p = 0;
    if buf.len() < 2 + 32 + 1 { return Err(TlsError::DecodeError); }
    p += 2;                                       // legacy_version
    let mut random = [0u8; 32];
    random.copy_from_slice(&buf[p..p + 32]);
    p += 32;

    // legacy_session_id
    let sid_len = buf[p] as usize; p += 1;
    if p + sid_len > buf.len() { return Err(TlsError::DecodeError); }
    let legacy_session_id = &buf[p..p + sid_len];
    p += sid_len;

    // cipher_suites
    if p + 2 > buf.len() { return Err(TlsError::DecodeError); }
    let cs_len = u16::from_be_bytes([buf[p], buf[p+1]]) as usize; p += 2;
    if p + cs_len > buf.len() { return Err(TlsError::DecodeError); }
    let cipher_suites = &buf[p..p + cs_len];
    p += cs_len;

    // legacy_compression_methods
    if p + 1 > buf.len() { return Err(TlsError::DecodeError); }
    let comp_len = buf[p] as usize; p += 1 + comp_len;

    // extensions
    let mut supports_tls13   = false;
    let mut key_share_x25519: Option<[u8; 32]> = None;
    if p + 2 <= buf.len() {
        let ext_total = u16::from_be_bytes([buf[p], buf[p+1]]) as usize; p += 2;
        let ext_end = p + ext_total;
        if ext_end > buf.len() { return Err(TlsError::DecodeError); }
        while p + 4 <= ext_end {
            let ext_type = u16::from_be_bytes([buf[p], buf[p+1]]); p += 2;
            let ext_len  = u16::from_be_bytes([buf[p], buf[p+1]]) as usize; p += 2;
            if p + ext_len > ext_end { return Err(TlsError::DecodeError); }
            let ext_body = &buf[p..p + ext_len];
            match ext_type {
                EXT_SUPPORTED_VERSIONS => {
                    let mut q = 1;
                    while q + 2 <= ext_body.len() {
                        if ext_body[q] == 0x03 && ext_body[q+1] == 0x04 {
                            supports_tls13 = true;
                        }
                        q += 2;
                    }
                }
                EXT_KEY_SHARE => {
                    let mut q = 2;          // skip client_shares length
                    while q + 4 <= ext_body.len() {
                        let group = u16::from_be_bytes([ext_body[q], ext_body[q+1]]);
                        let kex_len = u16::from_be_bytes([ext_body[q+2], ext_body[q+3]]) as usize;
                        if q + 4 + kex_len > ext_body.len() { break; }
                        if group == GROUP_X25519 && kex_len == 32 {
                            let mut k = [0u8; 32];
                            k.copy_from_slice(&ext_body[q+4..q+4+32]);
                            key_share_x25519 = Some(k);
                        }
                        q += 4 + kex_len;
                    }
                }
                _ => {}
            }
            p += ext_len;
        }
    }
    Ok(ClientHello {
        random, legacy_session_id, cipher_suites,
        key_share_x25519, supports_tls13,
    })
}

/// Build a ServerHello body — re-exported from stage-5 unchanged.
pub fn server_hello_into(
    out: &mut [u8],
    server_random:    &[u8; 32],
    legacy_session_id:&[u8],
    server_pubkey:    &[u8; 32],
) -> Result<usize, TlsError> {
    if legacy_session_id.len() > 32 { return Err(TlsError::DecodeError); }
    let ext_len = 2 + 2 + 2 + 2 + 2 + 2 + 2 + 32;
    let total   = 2 + 32 + 1 + legacy_session_id.len() + 2 + 1 + 2 + ext_len;
    if out.len() < total { return Err(TlsError::DecodeError); }

    let mut p = 0;
    out[p..p+2].copy_from_slice(&[0x03, 0x03]); p += 2;
    out[p..p+32].copy_from_slice(server_random); p += 32;
    out[p] = legacy_session_id.len() as u8; p += 1;
    out[p..p + legacy_session_id.len()].copy_from_slice(legacy_session_id);
    p += legacy_session_id.len();
    out[p..p+2].copy_from_slice(&[0x13, 0x03]); p += 2;
    out[p] = 0x00; p += 1;
    out[p..p+2].copy_from_slice(&(ext_len as u16).to_be_bytes()); p += 2;

    out[p..p+2].copy_from_slice(&[0x00, 0x2B]); p += 2;
    out[p..p+2].copy_from_slice(&[0x00, 0x02]); p += 2;
    out[p..p+2].copy_from_slice(&[0x03, 0x04]); p += 2;
    out[p..p+2].copy_from_slice(&[0x00, 0x33]); p += 2;
    out[p..p+2].copy_from_slice(&[0x00, 0x24]); p += 2;
    out[p..p+2].copy_from_slice(&[0x00, 0x1D]); p += 2;
    out[p..p+2].copy_from_slice(&[0x00, 0x20]); p += 2;
    out[p..p+32].copy_from_slice(server_pubkey); p += 32;

    Ok(p)
}
