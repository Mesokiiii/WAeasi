//! GDB Remote Serial Protocol stub.
//!
//! Wire format:
//! ```text
//!   $<payload>#<csum2>
//! ```
//! where `<csum2>` is two hex digits = byte-sum of payload mod 256.
//! The host sends one packet, we reply with one packet.
pub mod packet;
pub mod protocol;
pub mod regs;
pub mod reply;

pub use protocol::Stub;
