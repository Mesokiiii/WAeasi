//! Wire-protocol parser + reply formatter.
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct Request { pub verb: String, pub args: Vec<String> }

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError { Empty, BadEncoding }

pub fn parse_line(line: &str) -> Result<Request, ParseError> {
    let line = line.trim_end_matches(|c: char| c == '\r' || c == '\n');
    if line.is_empty() { return Err(ParseError::Empty); }
    let mut parts = line.split_whitespace();
    let verb = parts.next().ok_or(ParseError::Empty)?.to_uppercase();
    let args = parts.map(String::from).collect();
    Ok(Request { verb, args })
}

/// Standard error reply.
pub fn err(msg: &str) -> String {
    let mut s = String::with_capacity(msg.len() + 5);
    s.push_str("ERR ");
    s.push_str(msg);
    s.push('\n');
    s
}

pub fn ok() -> String { String::from("OK\n") }
