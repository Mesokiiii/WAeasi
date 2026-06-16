//! Wasm interpreter — direct-threaded MVP.
pub mod call_indirect;
pub mod control;
pub mod dispatch;
pub mod frame;
pub mod memory;
pub mod numeric;
pub mod opcode;
pub mod stack;
pub mod value;

pub use call_indirect::{FuncRef, FuncTable};
pub use dispatch::run;
pub use stack::ValueStack;
pub use value::{Cell, Value};
