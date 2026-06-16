//! TLS 1.3 alert protocol (RFC 8446 § 6).
//!
//! Two-byte messages: `[level | description]`.  TLS 1.3 effectively
//! treats every alert as `fatal`.
use super::TlsError;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlertLevel { Warning = 1, Fatal = 2 }

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AlertDescription {
    CloseNotify              = 0,
    UnexpectedMessage        = 10,
    BadRecordMac             = 20,
    HandshakeFailure         = 40,
    BadCertificate           = 42,
    UnsupportedCertificate   = 43,
    CertificateRevoked       = 44,
    CertificateExpired       = 45,
    CertificateUnknown       = 46,
    IllegalParameter         = 47,
    UnknownCa                = 48,
    DecodeError              = 50,
    DecryptError             = 51,
    ProtocolVersion          = 70,
    InsufficientSecurity     = 71,
    InternalError            = 80,
    InappropriateFallback    = 86,
    UserCanceled             = 90,
    MissingExtension         = 109,
    UnsupportedExtension     = 110,
    UnrecognizedName         = 112,
    BadCertificateStatusResponse = 113,
    UnknownPskIdentity       = 115,
    CertificateRequired      = 116,
    NoApplicationProtocol    = 120,
}

impl AlertDescription {
    /// Total-table decode — never UB.  Unknown values map to `DecodeError`
    /// (RFC 8446 § 6.2 instructs us to send a `decode_error` for any
    /// alert we can't understand and abort the connection).
    pub fn from_u8(b: u8) -> Self {
        match b {
            0   => Self::CloseNotify,
            10  => Self::UnexpectedMessage,
            20  => Self::BadRecordMac,
            40  => Self::HandshakeFailure,
            42  => Self::BadCertificate,
            43  => Self::UnsupportedCertificate,
            44  => Self::CertificateRevoked,
            45  => Self::CertificateExpired,
            46  => Self::CertificateUnknown,
            47  => Self::IllegalParameter,
            48  => Self::UnknownCa,
            50  => Self::DecodeError,
            51  => Self::DecryptError,
            70  => Self::ProtocolVersion,
            71  => Self::InsufficientSecurity,
            80  => Self::InternalError,
            86  => Self::InappropriateFallback,
            90  => Self::UserCanceled,
            109 => Self::MissingExtension,
            110 => Self::UnsupportedExtension,
            112 => Self::UnrecognizedName,
            113 => Self::BadCertificateStatusResponse,
            115 => Self::UnknownPskIdentity,
            116 => Self::CertificateRequired,
            120 => Self::NoApplicationProtocol,
            _   => Self::DecodeError,
        }
    }
}

pub fn build(level: AlertLevel, desc: AlertDescription) -> [u8; 2] {
    [level as u8, desc as u8]
}

pub fn parse(buf: &[u8]) -> Result<(AlertLevel, AlertDescription), TlsError> {
    if buf.len() != 2 { return Err(TlsError::DecodeError); }
    let level = match buf[0] {
        1 => AlertLevel::Warning,
        2 => AlertLevel::Fatal,
        _ => return Err(TlsError::DecodeError),
    };
    Ok((level, AlertDescription::from_u8(buf[1])))
}

/// Map a `TlsError` to the on-the-wire alert description we should send.
pub fn map(err: TlsError) -> AlertDescription {
    match err {
        TlsError::UnexpectedMessage => AlertDescription::UnexpectedMessage,
        TlsError::DecodeError       => AlertDescription::DecodeError,
        TlsError::BadRecordMac      => AlertDescription::BadRecordMac,
        TlsError::HandshakeFailure  => AlertDescription::HandshakeFailure,
        TlsError::Unsupported       => AlertDescription::ProtocolVersion,
        TlsError::Closed            => AlertDescription::CloseNotify,
    }
}
