//! HPACK (RFC 7541) — header compression for HTTP/2.
pub mod decoder;
pub mod dfa;
pub mod huffman;
pub mod huffman_table;
pub mod static_table;

pub use decoder::{Decoder, DecodeError};
