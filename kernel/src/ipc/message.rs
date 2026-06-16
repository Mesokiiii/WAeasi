//! Capability-routed binary message — analogous to seL4 endpoints.
use alloc::vec::Vec;

#[derive(Debug)]
pub struct Message {
    pub from: u64, // sender component id
    pub to:   u64, // recipient capability tag id
    pub body: Vec<u8>,
}

impl Message {
    pub fn new(from: u64, to: u64, body: Vec<u8>) -> Self {
        Self { from, to, body }
    }
    pub fn len(&self) -> usize { self.body.len() }
    pub fn is_empty(&self) -> bool { self.body.is_empty() }
}
