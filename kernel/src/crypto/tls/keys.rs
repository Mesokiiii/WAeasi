//! TLS 1.3 key schedule (RFC 8446 § 7).
//!
//! ```text
//!   PSK-or-zero ─► HKDF-Extract ─► early_secret
//!         │
//!         └─► HKDF-Expand-Label("derived", "") ─► derived
//!                                                 │
//!                ECDHE-or-zero ─► HKDF-Extract ─► handshake_secret
//!                                                 │
//!                                       ┌─────────┴─────────┐
//!                                       ▼                   ▼
//!                                  client_hs           server_hs
//!                                       │                   │
//!                                       └─► HKDF-Expand-Label("derived","") ─► derived
//!                                                                                │
//!                                                            zero ─► HKDF-Extract ─► master_secret
//! ```
use crate::crypto::hkdf;
use crate::crypto::sha256;

const HASH_LEN: usize = 32;

#[derive(Default, Debug)]
pub struct KeySchedule {
    pub early_secret:      [u8; HASH_LEN],
    pub handshake_secret:  [u8; HASH_LEN],
    pub master_secret:     [u8; HASH_LEN],
    pub client_hs_traffic: [u8; HASH_LEN],
    pub server_hs_traffic: [u8; HASH_LEN],
    pub client_app_traffic:[u8; HASH_LEN],
    pub server_app_traffic:[u8; HASH_LEN],
}

impl KeySchedule {
    /// Initialize from a transcript hash and an ECDHE shared secret.
    pub fn derive(
        transcript_hash: &[u8; HASH_LEN],
        ecdhe_shared:    &[u8; 32],
    ) -> Self {
        let mut s = Self::default();
        let zeros = [0u8; HASH_LEN];

        // Early secret = HKDF-Extract(salt=0, ikm=0).
        s.early_secret = hkdf::extract(&zeros, &zeros);

        // derived = HKDF-Expand-Label(early_secret, "derived", "", H("")).
        let empty_hash = sha256::hash(&[]);
        let mut derived = [0u8; HASH_LEN];
        hkdf::expand_label(&s.early_secret, "derived", &empty_hash, &mut derived);

        // handshake_secret = HKDF-Extract(salt=derived, ikm=ECDHE).
        s.handshake_secret = hkdf::extract(&derived, ecdhe_shared);

        // {client,server}_handshake_traffic_secret.
        hkdf::expand_label(&s.handshake_secret, "c hs traffic",
                           transcript_hash, &mut s.client_hs_traffic);
        hkdf::expand_label(&s.handshake_secret, "s hs traffic",
                           transcript_hash, &mut s.server_hs_traffic);

        // master_secret.
        let mut derived2 = [0u8; HASH_LEN];
        hkdf::expand_label(&s.handshake_secret, "derived", &empty_hash, &mut derived2);
        s.master_secret = hkdf::extract(&derived2, &zeros);

        s
    }

    /// Derive application traffic secrets from a finished-message transcript.
    pub fn application_traffic(&mut self, transcript_hash: &[u8; HASH_LEN]) {
        hkdf::expand_label(&self.master_secret, "c ap traffic",
                           transcript_hash, &mut self.client_app_traffic);
        hkdf::expand_label(&self.master_secret, "s ap traffic",
                           transcript_hash, &mut self.server_app_traffic);
    }

    /// Compute (key, iv) pair for a traffic secret, ChaCha20-Poly1305 sizes.
    pub fn traffic_keys(secret: &[u8; HASH_LEN]) -> ([u8; 32], [u8; 12]) {
        let mut key = [0u8; 32];
        let mut iv  = [0u8; 12];
        hkdf::expand_label(secret, "key", &[], &mut key);
        hkdf::expand_label(secret, "iv",  &[], &mut iv);
        (key, iv)
    }
}

/// Helper: compute SHA-256 over a sequence of buffers (transcript hash).
pub fn transcript_hash(parts: &[&[u8]]) -> [u8; HASH_LEN] {
    let mut h = sha256::Sha256::new();
    for p in parts { h.update(p); }
    h.finalize()
}
