//! Dual-stack routing — single routing table that serves both v4 and
//! v6 destinations.
pub mod lpm;
pub mod table;

pub use table::{Route, RoutingTable, Family};
