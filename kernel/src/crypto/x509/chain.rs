//! X.509 issuer-chain validation.
//!
//! Stage-7 algorithm (RFC 5280 § 6.1, simplified):
//!   1. The leaf certificate appears at index 0.
//!   2. For every i in `0..len-1`:
//!        * `cert[i].issuer_dn  == cert[i+1].subject_dn`
//!        * `cert[i].signature_alg.oid == cert[i+1].spki_alg.oid` (consistent)
//!        * `cert[i+1]` is within its `validity` window at `now_unix`.
//!   3. The last certificate is the trust anchor — its issuer must
//!      appear in the caller-provided `roots` set (subject-DN match).
//!
//! Cryptographic signature verification of each TBS is the next-stage
//! gate (`crypto::ed25519::verify` / `rsa::verify` once RSA lands in
//! Stage 8); chain *structure* is fully validated here.
use alloc::vec::Vec;

use super::cert::{Certificate, NameAttr, NameAttrOid};

#[derive(Debug, PartialEq, Eq)]
pub enum ChainError {
    Empty,
    IssuerSubjectMismatch(usize),
    AlgorithmMismatch(usize),
    NotYetValid(usize),
    Expired(usize),
    UntrustedRoot,
}

pub fn validate(
    chain:    &[Certificate<'_>],
    roots:    &[Vec<NameAttr>],
    now_unix: i64,
) -> Result<(), ChainError> {
    if chain.is_empty() { return Err(ChainError::Empty); }

    for (i, c) in chain.iter().enumerate() {
        if c.validity.not_before_unix > now_unix { return Err(ChainError::NotYetValid(i)); }
        if c.validity.not_after_unix  < now_unix { return Err(ChainError::Expired(i)); }
    }

    for i in 0..chain.len() - 1 {
        let child  = &chain[i];
        let parent = &chain[i + 1];
        if !dn_eq(&child.issuer_dn, &parent.subject_dn) {
            return Err(ChainError::IssuerSubjectMismatch(i));
        }
        if child.signature_alg.oid != parent.spki_alg.oid {
            return Err(ChainError::AlgorithmMismatch(i));
        }
    }

    let anchor_issuer = &chain[chain.len() - 1].issuer_dn;
    if !roots.iter().any(|r| dn_eq(anchor_issuer, r)) {
        return Err(ChainError::UntrustedRoot);
    }
    Ok(())
}

fn dn_eq(a: &[NameAttr], b: &[NameAttr]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).all(|(x, y)|
        x.oid_marker == y.oid_marker && x.value == y.value
    )
}

/// Convenience: extract the CN attribute from a DN, if present.
pub fn cn_of(dn: &[NameAttr]) -> Option<&str> {
    dn.iter().find(|a| a.oid_marker == NameAttrOid::CN).map(|a| a.value.as_str())
}
