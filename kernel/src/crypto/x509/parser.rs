//! Subset X.509 v3 certificate parser.
use alloc::string::String;
use alloc::vec::Vec;

use crate::crypto::asn1::der::{DerError, Reader, Tag};
use crate::crypto::asn1::oid::name_attr;

use super::cert::{AlgorithmId, Certificate, NameAttr, NameAttrOid, Validity};

#[derive(Debug, PartialEq, Eq)]
pub enum X509Error { Der(DerError), Unsupported }

impl From<DerError> for X509Error { fn from(e: DerError) -> Self { X509Error::Der(e) } }

pub fn parse(buf: &[u8]) -> Result<Certificate<'_>, X509Error> {
    let mut top = Reader::new(buf);
    let mut cert_seq = top.open_seq()?;

    // 1. tbsCertificate
    let tbs_start = cert_seq.pos();
    let mut tbs = cert_seq.open_seq()?;
    let raw_tbs = read_tbs_raw(buf, tbs_start)?;

    // 1a. version (optional context-specific [0])
    let _ = tbs.read_tlv()?;
    // 1b. serialNumber
    let serial_number = tbs.expect(Tag::INTEGER)?;
    // 1c. signature  AlgorithmIdentifier
    let signature_alg = parse_alg_id(&mut tbs)?;
    // 1d. issuer Name
    let issuer_dn = parse_name(&mut tbs)?;
    // 1e. validity
    let validity = parse_validity(&mut tbs)?;
    // 1f. subject Name
    let subject_dn = parse_name(&mut tbs)?;
    // 1g. subjectPublicKeyInfo
    let (spki_alg, spki_bits) = parse_spki(&mut tbs)?;

    // 2. signatureAlgorithm (must match)
    let _ = parse_alg_id(&mut cert_seq)?;
    // 3. signatureValue (BIT STRING)
    let sig_value = strip_bit_string(cert_seq.expect(Tag::BIT_STRING)?)?;

    Ok(Certificate {
        serial_number, signature_alg, issuer_dn, validity, subject_dn,
        spki_alg, spki_bits, signature_value: sig_value, raw_tbs,
    })
}

fn read_tbs_raw(buf: &[u8], start: usize) -> Result<&[u8], X509Error> {
    // Re-parse the SEQUENCE header to find its length so we can return
    // the canonical TBS slice (signature input).
    let mut r = Reader::new(&buf[start..]);
    let body = r.expect(Tag::SEQUENCE)?;
    let header_len = body.as_ptr() as usize - buf[start..].as_ptr() as usize;
    Ok(&buf[start..start + header_len + body.len()])
}

fn parse_alg_id<'a>(r: &mut Reader<'a>) -> Result<AlgorithmId<'a>, X509Error> {
    let mut alg = r.open_seq()?;
    let oid = alg.expect(Tag::OID)?;
    let params = if alg.eof() { None } else { Some(alg.read_tlv()?.1) };
    Ok(AlgorithmId { oid, parameters: params })
}

fn parse_name(r: &mut Reader<'_>) -> Result<Vec<NameAttr>, X509Error> {
    let mut name = r.open_seq()?;
    let mut out = Vec::new();
    while !name.eof() {
        let mut rdn = name.open_seq()?;          // RelativeDistinguishedName
        let mut attr = rdn.open_seq()?;          // AttributeTypeAndValue
        let oid = attr.expect(Tag::OID)?;
        let (_, val_bytes) = attr.read_tlv()?;
        let oid_marker = match oid {
            o if o == name_attr::CN => NameAttrOid::CN,
            o if o == name_attr::O  => NameAttrOid::O,
            o if o == name_attr::C  => NameAttrOid::C,
            _                       => NameAttrOid::Other,
        };
        out.push(NameAttr {
            oid_marker,
            value: String::from_utf8_lossy(val_bytes).into_owned(),
        });
    }
    Ok(out)
}

fn parse_validity(r: &mut Reader<'_>) -> Result<Validity, X509Error> {
    let mut v = r.open_seq()?;
    let (nb_tag, nb_bytes) = v.read_tlv()?;
    let (na_tag, na_bytes) = v.read_tlv()?;
    let nb = super::time::parse(nb_tag, nb_bytes)
        .map_err(|_| X509Error::Unsupported)?;
    let na = super::time::parse(na_tag, na_bytes)
        .map_err(|_| X509Error::Unsupported)?;
    Ok(Validity { not_before_unix: nb, not_after_unix: na })
}

fn parse_spki<'a>(r: &mut Reader<'a>) -> Result<(AlgorithmId<'a>, &'a [u8]), X509Error> {
    let mut spki = r.open_seq()?;
    let alg = parse_alg_id(&mut spki)?;
    let bits = strip_bit_string(spki.expect(Tag::BIT_STRING)?)?;
    Ok((alg, bits))
}

fn strip_bit_string(b: &[u8]) -> Result<&[u8], X509Error> {
    if b.is_empty() { return Err(X509Error::Unsupported); }
    if b[0] != 0    { return Err(X509Error::Unsupported); }   // unused-bits = 0
    Ok(&b[1..])
}
