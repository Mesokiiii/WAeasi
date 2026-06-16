//! SLAAC — Stateless Address Auto-Configuration (RFC 4862).
//!
//! State machine driven by Router Advertisements:
//!
//! ```text
//!   boot ──► link-local addr ──RS──► RA-with-prefix ──► tentative addr
//!                                                             │
//!                                       ┌─── DAD pass ────────┘
//!                                       ▼
//!                                  preferred ──pref-lifetime expire──► deprecated
//!                                       │                                   │
//!                                       └──── valid-lifetime expire ────────┘
//! ```
//!
//! Stage 8 implements the **address derivation** step (host part = EUI-64
//! from MAC, plus RFC 4941 stable-privacy variant) and the lifetime
//! tracker.  The DAD probe is wired to NDP's `solicit::build_into`.
use alloc::vec::Vec;

use crate::net::ethernet::MacAddr;
use crate::net::ipv6::Ipv6Addr;
use crate::net::ndp::router::PrefixInfo;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AddrState { Tentative, Preferred, Deprecated, Invalid }

#[derive(Copy, Clone, Debug)]
pub struct ConfiguredAddr {
    pub addr:               Ipv6Addr,
    pub prefix_length:      u8,
    pub state:              AddrState,
    pub preferred_until_tsc:u64,
    pub valid_until_tsc:    u64,
}

/// Generate a "Modified EUI-64" interface ID from a 48-bit MAC.
/// (RFC 4291 Appendix A.)  Flips the U/L bit and inserts `FFFE`.
pub fn iid_eui64(mac: &MacAddr) -> u64 {
    let m = mac.0;
    let bytes = [m[0] ^ 0x02, m[1], m[2], 0xFF, 0xFE, m[3], m[4], m[5]];
    u64::from_be_bytes(bytes)
}

/// RFC 7217 stable-privacy interface ID.  Stage 8 ships a deterministic
/// stable variant: `IID = SHA-256(secret || prefix || mac || dad_count)
/// >> truncate(64)`.  Re-deriving with a different `dad_count` after a
/// DAD collision keeps the address opaque.
pub fn iid_stable_privacy(secret: &[u8], prefix: &Ipv6Addr, mac: &MacAddr, dad: u8) -> u64 {
    use crate::crypto::sha256::Sha256;
    let mut h = Sha256::new();
    h.update(secret);
    h.update(&prefix.0[..8]);
    h.update(&mac.0);
    h.update(&[dad]);
    let d = h.finalize();
    u64::from_be_bytes(d[..8].try_into().unwrap())
}

/// Apply an inbound RA prefix; produce candidate ConfiguredAddrs.
pub fn process_prefix(
    info:       &PrefixInfo,
    mac:        &MacAddr,
    secret:     &[u8],
    now_tsc:    u64,
    tsc_per_sec:u64,
) -> Vec<ConfiguredAddr> {
    let mut out = Vec::new();
    if !info.autonomous           { return out; }
    if info.prefix_length != 64   { return out; }
    if info.valid_lifetime  == 0  { return out; }

    let preferred = now_tsc + info.preferred_lifetime as u64 * tsc_per_sec;
    let valid     = now_tsc + info.valid_lifetime    as u64 * tsc_per_sec;

    // EUI-64 variant.
    let iid = iid_eui64(mac);
    out.push(make_addr(&info.prefix, iid, info.prefix_length, preferred, valid));

    // Stable-privacy variant.
    let iid_priv = iid_stable_privacy(secret, &info.prefix, mac, 0);
    out.push(make_addr(&info.prefix, iid_priv, info.prefix_length, preferred, valid));
    out
}

fn make_addr(prefix: &Ipv6Addr, iid: u64, prefix_len: u8, pref: u64, valid: u64) -> ConfiguredAddr {
    let mut a = [0u8; 16];
    a[..8].copy_from_slice(&prefix.0[..8]);
    a[8..].copy_from_slice(&iid.to_be_bytes());
    ConfiguredAddr {
        addr: Ipv6Addr(a),
        prefix_length:       prefix_len,
        state:               AddrState::Tentative,
        preferred_until_tsc: pref,
        valid_until_tsc:     valid,
    }
}
