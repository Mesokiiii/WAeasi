//! ASN.1 / DER — bare-essentials decoder.
//!
//! The X.509 spec is large; we ship just what TLS 1.3 server-cert
//! parsing needs:
//!   * `der`  — TLV (tag-length-value) reader, primitive + constructed.
//!   * `oid`  — well-known OID byte slices (signature algorithms,
//!              public-key algorithms, name attributes).
pub mod der;
pub mod oid;

pub use der::{Reader, Tag};
