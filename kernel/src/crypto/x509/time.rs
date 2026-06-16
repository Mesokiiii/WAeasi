//! ASN.1 time parsing — UTCTime (RFC 5280 § 4.1.2.5.1) and
//! GeneralizedTime (§ 4.1.2.5.2).
//!
//! UTCTime (`YYMMDDhhmmssZ`):  2-digit year per RFC 5280 §4.1.2.5.1:
//!   * `YY ≥ 50` → 19YY
//!   * `YY < 50` → 20YY
//!
//! GeneralizedTime (`YYYYMMDDhhmmssZ`): 4-digit year — used for dates
//! at or after 2050.
//!
//! All times are UTC (`Z` suffix).  We refuse non-`Z` forms (no local
//! time, no fractional seconds for stage-9).
use crate::crypto::asn1::der::Tag;

#[derive(Debug, PartialEq, Eq)]
pub enum TimeError { BadFormat, BadDigit, OutOfRange }

/// Parse the body of an ASN.1 UTCTime or GeneralizedTime TLV.
/// Returns Unix seconds since 1970-01-01.  Stage-9 supports the
/// canonical RFC 5280 form `YY/YYYY MM DD hh mm ss Z`.
pub fn parse(tag: Tag, body: &[u8]) -> Result<i64, TimeError> {
    if tag == Tag::UTC_TIME { parse_utc(body) }
    else if tag == Tag::GENERALIZED_TIME { parse_generalized(body) }
    else { Err(TimeError::BadFormat) }
}

fn parse_utc(b: &[u8]) -> Result<i64, TimeError> {
    if b.len() != 13 || b[12] != b'Z' { return Err(TimeError::BadFormat); }
    let yy = digits_2(&b[0..2])?;
    let year = if yy >= 50 { 1900 + yy as i32 } else { 2000 + yy as i32 };
    let month = digits_2(&b[2..4])?;
    let day   = digits_2(&b[4..6])?;
    let hour  = digits_2(&b[6..8])?;
    let min   = digits_2(&b[8..10])?;
    let sec   = digits_2(&b[10..12])?;
    to_unix(year, month, day, hour, min, sec)
}

fn parse_generalized(b: &[u8]) -> Result<i64, TimeError> {
    if b.len() != 15 || b[14] != b'Z' { return Err(TimeError::BadFormat); }
    let year = digits_4(&b[0..4])?;
    let month = digits_2(&b[4..6])?;
    let day   = digits_2(&b[6..8])?;
    let hour  = digits_2(&b[8..10])?;
    let min   = digits_2(&b[10..12])?;
    let sec   = digits_2(&b[12..14])?;
    to_unix(year as i32, month, day, hour, min, sec)
}

#[inline]
fn digit(b: u8) -> Result<u32, TimeError> {
    if !b.is_ascii_digit() { return Err(TimeError::BadDigit); }
    Ok((b - b'0') as u32)
}
fn digits_2(b: &[u8]) -> Result<u32, TimeError> { Ok(digit(b[0])? * 10 + digit(b[1])?) }
fn digits_4(b: &[u8]) -> Result<u32, TimeError> {
    Ok(digit(b[0])? * 1000 + digit(b[1])? * 100 + digit(b[2])? * 10 + digit(b[3])?)
}

/// Convert calendar (year, month, day, hour, min, sec) to Unix epoch
/// seconds.  Algorithm: Howard Hinnant's "Date Algorithms" — branch-free,
/// works for any Gregorian date in the supported range.
fn to_unix(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32)
    -> Result<i64, TimeError>
{
    if !(1..=12).contains(&month) { return Err(TimeError::OutOfRange); }
    if !(1..=31).contains(&day)   { return Err(TimeError::OutOfRange); }
    if hour >= 24 || min >= 60 || sec >= 60 { return Err(TimeError::OutOfRange); }

    let y = year - if month <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32;
    let m_zero = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * m_zero + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era as i64 * 146097 + doe as i64 - 719468;

    Ok(days * 86_400 + hour as i64 * 3600 + min as i64 * 60 + sec as i64)
}
