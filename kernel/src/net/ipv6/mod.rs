//! IPv6 (RFC 8200) — fixed 40-byte header + extension chain.
//!
//! Layered:
//!   * `addr`       — `Ipv6Addr` utilities (classification, well-knowns).
//!   * `header`     — fixed-header parse / build + `Next Header` chain walker.
//!   * `extensions` — Hop-by-Hop, Routing, Destination Options, Fragment.
//!   * `fragment`   — reassembly state machine (RFC 8200 § 4.5).
pub mod addr;
pub mod extensions;
pub mod fragment;
pub mod header;

pub use addr::{Ipv6Addr, Scope};
pub use header::{Ipv6Header, NextHeader};

/// Standard `Next Header` numbers we care about — IANA Protocol Numbers.
pub mod proto {
    pub const HOP_BY_HOP:     u8 = 0;
    pub const ICMPV4:         u8 = 1;
    pub const TCP:            u8 = 6;
    pub const UDP:            u8 = 17;
    pub const ROUTING:        u8 = 43;
    pub const FRAGMENT:       u8 = 44;
    pub const ESP:            u8 = 50;
    pub const AH:             u8 = 51;
    pub const ICMPV6:         u8 = 58;
    pub const NO_NEXT:        u8 = 59;
    pub const DST_OPTIONS:    u8 = 60;
    pub const MOBILITY:       u8 = 135;
    pub const HIP:            u8 = 139;
    pub const SHIM6:          u8 = 140;
}
