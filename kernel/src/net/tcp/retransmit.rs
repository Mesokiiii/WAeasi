//! TCP retransmission timer (RFC 6298).
//!
//! Tracks RTT estimates (`srtt`, `rttvar`), computes the RTO, and keeps
//! a small queue of unacknowledged segments.  On each timer tick the
//! oldest expired segment is re-emitted; backoff doubles RTO up to a
//! 60-second cap (RFC 6298 § 5).
use alloc::collections::VecDeque;

const RTO_INIT_MS: u32 = 1000;
const RTO_MIN_MS:  u32 = 200;
const RTO_MAX_MS:  u32 = 60_000;
const ALPHA_NUM:   u32 = 1;   // α = 1/8 → smoothed_rtt
const ALPHA_DEN:   u32 = 8;
const BETA_NUM:    u32 = 1;   // β = 1/4 → rtt variation
const BETA_DEN:    u32 = 4;

#[derive(Debug, Clone, Copy)]
pub struct UnackedSeg {
    pub seq:    u32,
    pub len:    u32,
    pub sent_ms:u64,
    pub retx_n: u32,
}

#[derive(Debug)]
pub struct Retransmitter {
    pub srtt_ms:   Option<u32>,
    pub rttvar_ms: Option<u32>,
    pub rto_ms:    u32,
    pub queue:     VecDeque<UnackedSeg>,
}

impl Retransmitter {
    pub fn new() -> Self {
        Self {
            srtt_ms: None, rttvar_ms: None,
            rto_ms: RTO_INIT_MS, queue: VecDeque::new(),
        }
    }

    /// Track a freshly-sent segment.
    pub fn record_send(&mut self, seq: u32, len: u32, now_ms: u64) {
        self.queue.push_back(UnackedSeg { seq, len, sent_ms: now_ms, retx_n: 0 });
    }

    /// Drop segments fully covered by `ack` and update RTT estimators
    /// from the most recent ACK'd segment (Karn's algorithm: ignore RTT
    /// samples from retransmitted segments).
    pub fn on_ack(&mut self, ack: u32, now_ms: u64) {
        while let Some(s) = self.queue.front().copied() {
            if seq_le(s.seq.wrapping_add(s.len), ack) {
                if s.retx_n == 0 {
                    let rtt = (now_ms - s.sent_ms) as u32;
                    self.update_rtt(rtt);
                }
                self.queue.pop_front();
            } else { break; }
        }
    }

    /// Advance the timer.  Returns the segment to re-transmit, if any.
    pub fn tick(&mut self, now_ms: u64) -> Option<UnackedSeg> {
        let rto = self.rto_ms as u64;
        let s = self.queue.front_mut()?;
        if now_ms.saturating_sub(s.sent_ms) < rto { return None; }
        s.retx_n += 1;
        s.sent_ms = now_ms;
        // Exponential backoff capped at RTO_MAX.
        self.rto_ms = (self.rto_ms.saturating_mul(2)).min(RTO_MAX_MS);
        Some(*s)
    }

    fn update_rtt(&mut self, sample_ms: u32) {
        match (self.srtt_ms, self.rttvar_ms) {
            (Some(srtt), Some(rttvar)) => {
                let diff = if sample_ms > srtt { sample_ms - srtt } else { srtt - sample_ms };
                let new_rttvar = (rttvar * (BETA_DEN - BETA_NUM) + diff * BETA_NUM) / BETA_DEN;
                let new_srtt   = (srtt   * (ALPHA_DEN - ALPHA_NUM) + sample_ms * ALPHA_NUM) / ALPHA_DEN;
                self.rttvar_ms = Some(new_rttvar);
                self.srtt_ms   = Some(new_srtt);
                self.rto_ms    = (new_srtt + 4 * new_rttvar).max(RTO_MIN_MS).min(RTO_MAX_MS);
            }
            _ => {
                self.srtt_ms   = Some(sample_ms);
                self.rttvar_ms = Some(sample_ms / 2);
                self.rto_ms    = (sample_ms + 4 * sample_ms / 2).max(RTO_MIN_MS).min(RTO_MAX_MS);
            }
        }
    }
}

#[inline]
fn seq_le(a: u32, b: u32) -> bool { a.wrapping_sub(b) > i32::MAX as u32 || a == b }
