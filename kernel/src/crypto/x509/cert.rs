//! Decoded X.509 certificate types.
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct AlgorithmId<'a> {
    pub oid:        &'a [u8],
    pub parameters: Option<&'a [u8]>,
}

#[derive(Debug, Clone)]
pub struct Validity {
    pub not_before_unix: i64,
    pub not_after_unix:  i64,
}

#[derive(Debug, Clone)]
pub struct Certificate<'a> {
    pub serial_number:     &'a [u8],
    pub signature_alg:     AlgorithmId<'a>,
    pub issuer_dn:         Vec<NameAttr>,
    pub validity:          Validity,
    pub subject_dn:        Vec<NameAttr>,
    pub spki_alg:          AlgorithmId<'a>,
    pub spki_bits:         &'a [u8],   // raw subject public-key bytes
    pub signature_value:   &'a [u8],
    pub raw_tbs:           &'a [u8],   // for signature verification
}

#[derive(Debug, Clone)]
pub struct NameAttr {
    pub oid_marker: NameAttrOid,
    pub value:      String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameAttrOid { CN, O, C, Other }
