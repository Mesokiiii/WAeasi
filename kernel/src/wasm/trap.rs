//! Wasm trap representation.  Modeled after the canonical wasm-spec set.
#[derive(Debug, Copy, Clone)]
pub enum Trap {
    Unreachable,
    OutOfBounds,
    DivisionByZero,
    IntegerOverflow,
    InvalidConversion,
    StackOverflow,
    OutOfFuel,
    HostError,
}

impl Trap {
    pub fn message(self) -> &'static str {
        match self {
            Trap::Unreachable        => "unreachable instruction executed",
            Trap::OutOfBounds        => "memory access out of bounds",
            Trap::DivisionByZero     => "integer divide by zero",
            Trap::IntegerOverflow    => "integer overflow",
            Trap::InvalidConversion  => "invalid float-to-int conversion",
            Trap::StackOverflow      => "call stack exhausted",
            Trap::OutOfFuel          => "instance ran out of fuel",
            Trap::HostError          => "host function returned an error",
        }
    }
}
