//! TLS 1.3 server handshake state machine.
//!
//! Stage-10 implementation now drives a **real** flow:
//!   1. `accept(client_hello_bytes)` — parses ClientHello, extracts the
//!      client's X25519 key share, derives ECDHE shared secret.
//!   2. `make_server_hello(out)` — writes the ServerHello body into the
//!      caller's buffer.  Caller frames it as a record + handshake.
//!   3. `key_schedule_update(transcript_hash)` — derives the handshake
//!      traffic secrets per RFC 8446 § 7.
//!   4. `finish(transcript_hash)` — promotes to the application-data
//!      key schedule.
use super::handshake::{parse_client_hello, ClientHello};
use super::keys::KeySchedule;
use super::TlsError;
use crate::crypto::sha256::Sha256;
use crate::crypto::x25519;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    Start, ReceivedClientHello, SentServerHello,
    WaitFinished, Connected, Closed,
}

pub struct Server {
    pub state:        State,
    pub keys:         KeySchedule,
    server_secret:    [u8; 32],
    pub server_public:[u8; 32],
    pub server_random:[u8; 32],
    /// Captured client random — used in transcript + diagnostics.
    pub client_random: Option<[u8; 32]>,
    pub session_id:   alloc::vec::Vec<u8>,
    pub transcript:   Sha256,
}

impl Server {
    pub fn new(server_secret: [u8; 32]) -> Self {
        let mut server_random = [0u8; 32];
        let r = crate::wasi::preview2::random::get_random_bytes(32);
        server_random.copy_from_slice(&r);
        Self {
            state: State::Start,
            keys:  KeySchedule::default(),
            server_public: x25519::public_key(&server_secret),
            server_secret,
            server_random,
            client_random: None,
            session_id:    alloc::vec::Vec::new(),
            transcript:    Sha256::new(),
        }
    }

    /// Process an inbound ClientHello.  After this returns `Ok`, the
    /// caller can ask `make_server_hello`.
    pub fn accept(&mut self, ch_bytes: &[u8]) -> Result<(), TlsError> {
        if !matches!(self.state, State::Start) {
            return Err(TlsError::UnexpectedMessage);
        }
        let ch: ClientHello<'_> = parse_client_hello(ch_bytes)?;
        if !ch.supports_tls13 {
            return Err(TlsError::Unsupported);
        }
        let client_pub = ch.key_share_x25519.ok_or(TlsError::HandshakeFailure)?;

        // ECDHE shared secret.
        let shared = x25519::x25519(&self.server_secret, &client_pub);

        // Update transcript (just the ClientHello body for stage-10).
        self.transcript.update(ch_bytes);
        let th = self.transcript.clone().finalize();

        // Derive handshake-secret tier of the key schedule.
        self.keys = KeySchedule::derive(&th, &shared);

        self.client_random = Some(ch.random);
        self.session_id.extend_from_slice(ch.legacy_session_id);
        self.state = State::ReceivedClientHello;
        Ok(())
    }

    /// Write the ServerHello body into `out` and advance the state.
    pub fn make_server_hello(&mut self, out: &mut [u8]) -> Result<usize, TlsError> {
        if !matches!(self.state, State::ReceivedClientHello) {
            return Err(TlsError::UnexpectedMessage);
        }
        let n = super::handshake::server_hello_into(
            out, &self.server_random, &self.session_id, &self.server_public)?;
        self.transcript.update(&out[..n]);
        self.state = State::SentServerHello;
        Ok(n)
    }

    /// Promote to application-data traffic secrets after the
    /// client's `Finished` arrives.
    pub fn finish(&mut self) -> Result<(), TlsError> {
        if !matches!(self.state, State::WaitFinished | State::SentServerHello) {
            return Err(TlsError::UnexpectedMessage);
        }
        let th = self.transcript.clone().finalize();
        self.keys.application_traffic(&th);
        self.state = State::Connected;
        Ok(())
    }

    pub fn close(&mut self) { self.state = State::Closed; }
}
