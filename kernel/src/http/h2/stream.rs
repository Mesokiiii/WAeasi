//! HTTP/2 stream state (RFC 9113 § 5.1).
//!
//! ```text
//!   IDLE ──recv H──► OPEN ──recv H+ES──► HALF_CLOSED_REMOTE
//!         ──send H──►                    ──send H+ES──► CLOSED
//!                                                            │
//!   IDLE ──recv RST_STREAM──► CLOSED ◄──── any ────RST_STREAM ┘
//! ```
use super::frame::FrameType;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StreamState {
    Idle, Open, HalfClosedLocal, HalfClosedRemote,
    ReservedLocal, ReservedRemote, Closed,
}

#[derive(Debug)]
pub struct Stream {
    pub id:     u32,
    pub state:  StreamState,
    pub recv_window: i32,
    pub send_window: i32,
}

impl Stream {
    pub fn new(id: u32, initial_window: i32) -> Self {
        Self { id, state: StreamState::Idle, recv_window: initial_window, send_window: initial_window }
    }

    /// Single transition by inbound frame.  Returns the post-transition
    /// state (or Closed on protocol error).
    pub fn on_recv(&mut self, kind: FrameType, end_stream: bool) -> StreamState {
        use StreamState::*;
        self.state = match (self.state, kind) {
            (Idle,            FrameType::Headers)      => if end_stream { HalfClosedRemote } else { Open },
            (Open,            FrameType::Data)         => if end_stream { HalfClosedRemote } else { Open },
            (Open,            FrameType::Headers)      => if end_stream { HalfClosedRemote } else { Open },
            (HalfClosedLocal, FrameType::Data)         => if end_stream { Closed } else { HalfClosedLocal },
            (_,               FrameType::RstStream)    => Closed,
            (s, _)                                     => s,
        };
        self.state
    }

    pub fn on_send(&mut self, kind: FrameType, end_stream: bool) -> StreamState {
        use StreamState::*;
        self.state = match (self.state, kind) {
            (Idle,            FrameType::Headers)   => if end_stream { HalfClosedLocal } else { Open },
            (Open,            FrameType::Data)      => if end_stream { HalfClosedLocal } else { Open },
            (HalfClosedRemote,FrameType::Data)      => if end_stream { Closed } else { HalfClosedRemote },
            (_,               FrameType::RstStream) => Closed,
            (s, _)                                  => s,
        };
        self.state
    }
}
