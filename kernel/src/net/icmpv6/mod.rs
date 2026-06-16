//! ICMPv6 (RFC 4443).
//!
//! Mandatory in v6 — Path-MTU, NDP, MLD, error reporting all ride on
//! ICMPv6.  Layered:
//!   * `header` — 4-byte common header + pseudo-header checksum.
//!   * `echo`   — Echo Request/Reply (Type 128/129).
//!   * `error`  — Type 1..4 error messages (DstUnreach, PacketTooBig,
//!                TimeExceeded, ParamProblem).
//!
//! NDP messages live in `crate::net::ndp` since they are technically
//! ICMPv6 type 133-137 but with their own option layout.
pub mod echo;
pub mod error;
pub mod header;

pub use header::{IcmpV6Header, IcmpV6Type};

/// `Type` values per IANA.
pub mod ty {
    pub const DST_UNREACH:        u8 = 1;
    pub const PACKET_TOO_BIG:     u8 = 2;
    pub const TIME_EXCEEDED:      u8 = 3;
    pub const PARAM_PROBLEM:      u8 = 4;
    pub const ECHO_REQUEST:       u8 = 128;
    pub const ECHO_REPLY:         u8 = 129;
    pub const MLD_QUERY:          u8 = 130;
    pub const MLD_REPORT:         u8 = 131;
    pub const MLD_DONE:           u8 = 132;
    pub const ROUTER_SOLICIT:     u8 = 133;
    pub const ROUTER_ADVERT:      u8 = 134;
    pub const NEIGHBOR_SOLICIT:   u8 = 135;
    pub const NEIGHBOR_ADVERT:    u8 = 136;
    pub const REDIRECT:           u8 = 137;
    pub const MLD_REPORT_V2:      u8 = 143;
}
