//! HPACK Huffman decoder.
//!
//! Stage-7 routing:
//!   * If the supplied DFA table is non-empty, decode through the
//!     8-bit-per-step state machine (fast path).
//!   * Otherwise fall back to the bit-level scan over `huffman_table`.
use alloc::vec::Vec;

use super::dfa::{decode_with, SKELETON};
use super::huffman_table::TABLE;

#[derive(Debug, PartialEq, Eq)]
pub enum HuffError { TruncatedSymbol, InvalidPadding, UnknownCode }

pub fn decode(input: &[u8]) -> Result<Vec<u8>, HuffError> {
    if !SKELETON.table.is_empty() {
        return decode_with(&SKELETON, input).map_err(|_| HuffError::UnknownCode);
    }
    decode_bitlevel(input)
}

fn decode_bitlevel(input: &[u8]) -> Result<Vec<u8>, HuffError> {
    let mut out = Vec::with_capacity(input.len());
    let mut acc: u64 = 0;
    let mut acc_bits: u8 = 0;

    for &b in input {
        acc = (acc << 8) | (b as u64);
        acc_bits += 8;
        loop {
            if acc_bits == 0 { break; }
            match match_symbol(acc, acc_bits) {
                Some((sym, used)) => {
                    if sym == 256 { return Err(HuffError::UnknownCode); }
                    out.push(sym as u8);
                    acc &= (1u64 << (acc_bits - used)) - 1;
                    acc_bits -= used;
                }
                None => break,
            }
        }
    }
    if acc_bits > 7 { return Err(HuffError::InvalidPadding); }
    if acc_bits > 0 {
        let pad_mask = (1u64 << acc_bits) - 1;
        if acc != pad_mask { return Err(HuffError::InvalidPadding); }
    }
    Ok(out)
}

#[inline]
fn match_symbol(acc: u64, acc_bits: u8) -> Option<(u32, u8)> {
    for (sym, &(code, bits)) in TABLE.iter().enumerate() {
        if acc_bits < bits { continue; }
        let high = (acc >> (acc_bits - bits)) as u32;
        if high == code { return Some((sym as u32, bits)); }
    }
    None
}
