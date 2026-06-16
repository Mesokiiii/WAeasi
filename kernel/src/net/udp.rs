//! UDP — datagram queue per socket.
use alloc::collections::VecDeque;
use alloc::vec::Vec;

pub struct UdpDatagram {
    pub src_ip:   [u8; 16],
    pub src_port: u16,
    pub data:     Vec<u8>,
}

pub struct UdpSocket {
    pub local_port: u16,
    pub queue:      VecDeque<UdpDatagram>,
}

impl UdpSocket {
    pub fn new() -> Self {
        Self { local_port: 0, queue: VecDeque::new() }
    }
    pub fn bind(&mut self, port: u16) { self.local_port = port; }
    pub fn recv(&mut self) -> Option<UdpDatagram> { self.queue.pop_front() }
}
