//! TCP connection state machine (RFC 9293).
//!
//! Stage-9 hardening:
//!   * RX/TX queues are fixed-capacity `RxRing`s — no per-segment
//!     `VecDeque::extend` reallocation, bounded memory per conn.
//!   * `rx_window()` reflects actual ring free space → correct
//!     advertised window without wraparound surprises.
use super::rxring::RxRing;
use super::segment::{Flags, Segment};
use crate::net::sockaddr::SocketAddr;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TcpState {
    Closed, Listen, SynSent, SynReceived,
    Established, FinWait1, FinWait2, CloseWait, Closing, LastAck, TimeWait,
}

#[derive(Debug)]
pub struct TcpConnection {
    pub state:    TcpState,
    pub local:    Option<SocketAddr>,
    pub remote:   Option<SocketAddr>,
    pub iss:      u32,
    pub snd_nxt:  u32,
    pub snd_una:  u32,
    pub rcv_nxt:  u32,
    pub rx:       RxRing,
    pub tx:       RxRing,
}

impl TcpConnection {
    pub fn new() -> Self {
        let iss = crate::wasi::preview2::random::get_random_u64() as u32;
        Self {
            state: TcpState::Closed,
            local: None, remote: None,
            iss,
            snd_nxt: iss, snd_una: iss, rcv_nxt: 0,
            rx: RxRing::new(), tx: RxRing::new(),
        }
    }

    pub fn listen(&mut self, addr: SocketAddr) {
        self.local = Some(addr);
        self.state = TcpState::Listen;
    }

    pub fn is_v6(&self) -> bool {
        matches!(self.local, Some(SocketAddr::V6(_)))
    }

    /// Single-step state machine.  Returns flags for the immediate reply,
    /// or `None` if the segment is dropped (out-of-window or unexpected).
    pub fn step(&mut self, seg: &Segment<'_>) -> Option<Flags> {
        let f = seg.header.flags();

        // Window check — only meaningful past LISTEN.
        if !matches!(self.state, TcpState::Closed | TcpState::Listen | TcpState::SynSent)
           && !self.in_window(seg.header.seq_host(), seg.payload.len() as u32)
        {
            return None;
        }

        match self.state {
            TcpState::Listen => {
                if f.contains(Flags::SYN) && !f.contains(Flags::ACK) {
                    self.rcv_nxt = seg.header.seq_host().wrapping_add(1);
                    self.snd_nxt = self.snd_nxt.wrapping_add(1);
                    self.state = TcpState::SynReceived;
                    return Some(Flags::SYN | Flags::ACK);
                }
            }
            TcpState::SynReceived => {
                if f.contains(Flags::ACK) {
                    self.snd_una = seg.header.ack_host();
                    self.state = TcpState::Established;
                }
            }
            TcpState::Established => {
                if !seg.payload.is_empty() {
                    let pushed = self.rx.push(seg.payload);
                    if pushed > 0 {
                        self.rcv_nxt = self.rcv_nxt.wrapping_add(pushed as u32);
                        return Some(Flags::ACK);
                    }
                    // Ring full — drop, no ACK; peer will retransmit
                    // once our window opens.
                    return None;
                }
                if f.contains(Flags::FIN) {
                    self.rcv_nxt = self.rcv_nxt.wrapping_add(1);
                    self.state = TcpState::CloseWait;
                    return Some(Flags::ACK);
                }
            }
            TcpState::FinWait1 => {
                if f.contains(Flags::FIN | Flags::ACK) {
                    self.state = TcpState::TimeWait;
                    return Some(Flags::ACK);
                }
                if f.contains(Flags::ACK) { self.state = TcpState::FinWait2; }
            }
            TcpState::LastAck => {
                if f.contains(Flags::ACK) { self.state = TcpState::Closed; }
            }
            _ => {}
        }
        None
    }

    fn in_window(&self, seg_seq: u32, seg_len: u32) -> bool {
        let lo = self.rcv_nxt;
        let hi = lo.wrapping_add(self.rx.advertised_window() as u32);
        seq_in(seg_seq, lo, hi) ||
        (seg_len > 0 && seq_in(seg_seq.wrapping_add(seg_len - 1), lo, hi))
    }

    pub fn close(&mut self) {
        self.state = match self.state {
            TcpState::Established => TcpState::FinWait1,
            TcpState::CloseWait   => TcpState::LastAck,
            other                 => other,
        };
    }

    /// Bytes the peer is still allowed to send.
    pub fn rx_window(&self) -> u32 { self.rx.advertised_window() as u32 }
}

#[inline]
fn seq_in(x: u32, lo: u32, hi: u32) -> bool {
    x.wrapping_sub(lo) < hi.wrapping_sub(lo)
}
