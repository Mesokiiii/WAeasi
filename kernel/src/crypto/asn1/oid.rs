//! Well-known OIDs as raw DER-encoded byte slices.
//!
//! An OID's first two arcs are encoded as `40·a + b`, then each
//! subsequent arc is base-128 little-endian-end with continuation bits.
//! We keep them as `&[u8]` so comparison is `==`.
pub mod sig {
    /// `1.3.101.112` — Ed25519 signature.
    pub const ED25519:           &[u8] = &[0x2B, 0x65, 0x70];
    /// `1.2.840.113549.1.1.11` — sha256WithRSAEncryption.
    pub const RSA_SHA256:        &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B];
    /// `1.2.840.10045.4.3.2`   — ecdsa-with-SHA256.
    pub const ECDSA_SHA256:      &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x04, 0x03, 0x02];
}

pub mod pubkey {
    /// `1.3.101.112` — Ed25519 public key.
    pub const ED25519:    &[u8] = &[0x2B, 0x65, 0x70];
    /// `1.2.840.113549.1.1.1` — rsaEncryption.
    pub const RSA:        &[u8] = &[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x01];
    /// `1.2.840.10045.2.1` — id-ecPublicKey.
    pub const EC:         &[u8] = &[0x2A, 0x86, 0x48, 0xCE, 0x3D, 0x02, 0x01];
}

pub mod name_attr {
    /// `2.5.4.3` — CN (commonName).
    pub const CN:         &[u8] = &[0x55, 0x04, 0x03];
    /// `2.5.4.10` — O (organizationName).
    pub const O:          &[u8] = &[0x55, 0x04, 0x0A];
    /// `2.5.4.6` — C (countryName).
    pub const C:          &[u8] = &[0x55, 0x04, 0x06];
}

pub mod ext {
    /// `2.5.29.17` — subjectAltName.
    pub const SAN:        &[u8] = &[0x55, 0x1D, 0x11];
    /// `2.5.29.19` — basicConstraints.
    pub const BASIC_CONSTRAINTS: &[u8] = &[0x55, 0x1D, 0x13];
    /// `2.5.29.15` — keyUsage.
    pub const KEY_USAGE:  &[u8] = &[0x55, 0x1D, 0x0F];
    /// `2.5.29.37` — extendedKeyUsage.
    pub const EXT_KEY_USAGE: &[u8] = &[0x55, 0x1D, 0x25];
}
