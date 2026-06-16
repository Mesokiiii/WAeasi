//! WASI Preview 2 host functions.  Each interface lives in its own file so
//! none of them grow past the 250-LoC budget.
pub mod clocks;
pub mod filesystem;
pub mod io;
pub mod random;
pub mod sockets;
