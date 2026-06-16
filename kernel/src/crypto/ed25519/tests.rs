//! RFC 8032 § 7.1 test vectors — runtime self-test.
//!
//! Boots a small known-answer harness that validates `keypair`, `sign`,
//! and `verify` against the spec.  Failures are escalated as
//! `panic!` — Ed25519 is in the trusted-compute-base, so a botched
//! constant must never be silently tolerated.
use super::keypair::{keypair, sign, verify, PublicKey, Signature};

/// `(seed, msg, expected_pub, expected_sig)` per RFC 8032 § 7.1.
struct Vector {
    seed:     [u8; 32],
    msg:      &'static [u8],
    pub_key:  PublicKey,
    sig:      Signature,
}

const VECTORS: &[Vector] = &[
    // TEST 1 — empty message.
    Vector {
        seed:    hex32("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"),
        msg:     &[],
        pub_key: hex32("d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"),
        sig:     hex64(concat!(
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155",
            "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        )),
    },
    // TEST 2 — single-byte message.
    Vector {
        seed:    hex32("4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"),
        msg:     &[0x72],
        pub_key: hex32("3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c"),
        sig:     hex64(concat!(
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da",
            "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        )),
    },
];

/// Run all vectors.  Returns `Ok(())` on success or `Err(name)` for the
/// first failing vector.
pub fn self_test() -> Result<(), &'static str> {
    for (i, v) in VECTORS.iter().enumerate() {
        let derived = keypair(&v.seed);
        if derived != v.pub_key { return Err(name_for(i, "pub_key mismatch")); }
        let sig = sign(&v.seed, v.msg);
        if sig != v.sig { return Err(name_for(i, "sig mismatch")); }
        if !verify(&v.pub_key, v.msg, &v.sig) { return Err(name_for(i, "verify rejected own sig")); }
    }
    log::info!("[ed25519] self-test passed: {} vector(s)", VECTORS.len());
    Ok(())
}

const fn name_for(i: usize, _msg: &'static str) -> &'static str {
    // Stage-7 returns a coarse pointer; stage-8 will format the actual
    // index + reason via a kernel-side stack-allocated formatter.
    match i {
        0 => "rfc8032 vector 1",
        1 => "rfc8032 vector 2",
        _ => "rfc8032 unknown",
    }
}

const fn hex32(s: &'static str) -> [u8; 32] {
    let bytes = s.as_bytes();
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (hex_nib(bytes[i*2]) << 4) | hex_nib(bytes[i*2 + 1]);
        i += 1;
    }
    out
}

const fn hex64(s: &'static str) -> [u8; 64] {
    let bytes = s.as_bytes();
    let mut out = [0u8; 64];
    let mut i = 0;
    while i < 64 {
        out[i] = (hex_nib(bytes[i*2]) << 4) | hex_nib(bytes[i*2 + 1]);
        i += 1;
    }
    out
}

const fn hex_nib(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}
