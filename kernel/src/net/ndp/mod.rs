//! Neighbor Discovery Protocol (RFC 4861).
//!
//! NDP replaces ARP for IPv6.  Five message types, all carried as ICMPv6:
//!   * `solicit` — Neighbor Solicitation (NS, type 135).
//!   * `advert`  — Neighbor Advertisement (NA, type 136).
//!   * `router`  — Router Solicitation / Advertisement (RS=133, RA=134).
//!   * `redirect`— Redirect (type 137).
//!
//! Plus the `cache` (Neighbor Cache) — five-state machine
//! INCOMPLETE → REACHABLE → STALE → DELAY → PROBE.
pub mod advert;
pub mod cache;
pub mod redirect;
pub mod router;
pub mod solicit;

pub use cache::{NeighborCache, NeighborState, neighbor_cache};

/// NDP options (RFC 4861 § 4.6) — TLV with `kind u8 | length_units u8 | value`.
/// `length_units` measures (kind+length+value) in 8-byte chunks.
pub mod opt {
    pub const SOURCE_LINK_LAYER:    u8 = 1;
    pub const TARGET_LINK_LAYER:    u8 = 2;
    pub const PREFIX_INFORMATION:   u8 = 3;
    pub const REDIRECTED_HEADER:    u8 = 4;
    pub const MTU:                  u8 = 5;
}
