//! Wasm opcode constants — exhaustive for the MVP subset we execute.
//!
//! Keeping them in one place makes the dispatcher table more readable
//! and prevents the magic-number drift between parser/validator/interp.
#![allow(non_upper_case_globals)]

pub const UNREACHABLE:  u8 = 0x00;
pub const NOP:          u8 = 0x01;
pub const BLOCK:        u8 = 0x02;
pub const LOOP:         u8 = 0x03;
pub const IF_:          u8 = 0x04;
pub const ELSE_:        u8 = 0x05;
pub const END:          u8 = 0x0B;
pub const BR:           u8 = 0x0C;
pub const BR_IF:        u8 = 0x0D;
pub const RETURN:       u8 = 0x0F;
pub const CALL:         u8 = 0x10;
pub const DROP:         u8 = 0x1A;
pub const SELECT:       u8 = 0x1B;

pub const LOCAL_GET:    u8 = 0x20;
pub const LOCAL_SET:    u8 = 0x21;
pub const LOCAL_TEE:    u8 = 0x22;

pub const I32_LOAD:     u8 = 0x28;
pub const I32_STORE:    u8 = 0x36;
pub const MEMORY_SIZE:  u8 = 0x3F;
pub const MEMORY_GROW:  u8 = 0x40;

pub const I32_CONST:    u8 = 0x41;
pub const I64_CONST:    u8 = 0x42;

// Numeric — i32
pub const I32_EQZ:  u8 = 0x45;
pub const I32_EQ:   u8 = 0x46; pub const I32_NE: u8 = 0x47;
pub const I32_LT_S: u8 = 0x48; pub const I32_LT_U: u8 = 0x49;
pub const I32_GT_S: u8 = 0x4A; pub const I32_GT_U: u8 = 0x4B;
pub const I32_LE_S: u8 = 0x4C; pub const I32_LE_U: u8 = 0x4D;
pub const I32_GE_S: u8 = 0x4E; pub const I32_GE_U: u8 = 0x4F;
pub const I32_ADD:  u8 = 0x6A; pub const I32_SUB: u8 = 0x6B;
pub const I32_MUL:  u8 = 0x6C;
pub const I32_DIV_S: u8 = 0x6D; pub const I32_DIV_U: u8 = 0x6E;
pub const I32_REM_S: u8 = 0x6F; pub const I32_REM_U: u8 = 0x70;
pub const I32_AND:  u8 = 0x71; pub const I32_OR:  u8 = 0x72;
pub const I32_XOR:  u8 = 0x73; pub const I32_SHL: u8 = 0x74;
pub const I32_SHR_S: u8 = 0x75; pub const I32_SHR_U: u8 = 0x76;
pub const I32_ROTL: u8 = 0x77; pub const I32_ROTR: u8 = 0x78;
