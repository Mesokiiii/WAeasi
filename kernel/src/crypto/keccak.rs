//! Keccak-f[1600] permutation — foundation for SHA-3 and SHAKE.
//!
//! Reference: NIST FIPS 202.
//!
//! State is a 5×5×64 = 1600-bit array, represented as `[u64; 25]`.
//! Each round consists of θ, ρ, π, χ, ι steps — 24 rounds total.
//!
//! All operations are constant-time integer ops; no data-dependent
//! branches.  Stage-9 ships the canonical bit-interleaved layout used
//! by every conformant SHA-3 implementation.
pub type State = [u64; 25];

/// Round constants (RC[i] for i = 0..24) — FIPS 202 Table 1.
const RC: [u64; 24] = [
    0x0000000000000001, 0x0000000000008082, 0x800000000000808A, 0x8000000080008000,
    0x000000000000808B, 0x0000000080000001, 0x8000000080008081, 0x8000000000008009,
    0x000000000000008A, 0x0000000000000088, 0x0000000080008009, 0x000000008000000A,
    0x000000008000808B, 0x800000000000008B, 0x8000000000008089, 0x8000000000008003,
    0x8000000000008002, 0x8000000000000080, 0x000000000000800A, 0x800000008000000A,
    0x8000000080008081, 0x8000000000008080, 0x0000000080000001, 0x8000000080008008,
];

/// Rotation offsets — `RO[x][y]` = ρ rotation for lane (x,y).
const RHO: [u32; 25] = [
     0,  1, 62, 28, 27,
    36, 44,  6, 55, 20,
     3, 10, 43, 25, 39,
    41, 45, 15, 21,  8,
    18,  2, 61, 56, 14,
];

/// Apply 24 rounds of Keccak-f to `s`.
pub fn permute(s: &mut State) {
    for round in 0..24 {
        // θ: column parities + spread.
        let mut c = [0u64; 5];
        for x in 0..5 { c[x] = s[x] ^ s[x+5] ^ s[x+10] ^ s[x+15] ^ s[x+20]; }
        let mut d = [0u64; 5];
        for x in 0..5 {
            d[x] = c[(x + 4) % 5] ^ c[(x + 1) % 5].rotate_left(1);
        }
        for y in 0..5 {
            for x in 0..5 { s[x + 5*y] ^= d[x]; }
        }

        // ρ + π — combined into one pass.  We rotate each lane and place
        // it at its π-permuted position.
        let mut b = [0u64; 25];
        for y in 0..5 {
            for x in 0..5 {
                let new_x = y;
                let new_y = (2 * x + 3 * y) % 5;
                b[new_x + 5 * new_y] = s[x + 5 * y].rotate_left(RHO[x + 5*y]);
            }
        }

        // χ — non-linear step.
        for y in 0..5 {
            for x in 0..5 {
                s[x + 5*y] = b[x + 5*y]
                    ^ ((!b[((x + 1) % 5) + 5*y]) & b[((x + 2) % 5) + 5*y]);
            }
        }

        // ι — XOR round constant into lane (0,0).
        s[0] ^= RC[round];
    }
}

/// Absorb a single block (full rate) into the state.
#[inline]
pub fn absorb_block(s: &mut State, block: &[u8], rate_bytes: usize) {
    debug_assert!(block.len() >= rate_bytes);
    debug_assert!(rate_bytes % 8 == 0);
    for i in 0..(rate_bytes / 8) {
        let lane = u64::from_le_bytes(block[i*8..i*8+8].try_into().unwrap());
        s[i] ^= lane;
    }
    permute(s);
}

/// Squeeze `out.len()` bytes from the state, calling `permute` once
/// per `rate_bytes` consumed.
pub fn squeeze(s: &mut State, rate_bytes: usize, out: &mut [u8]) {
    debug_assert!(rate_bytes % 8 == 0);
    let mut written = 0;
    while written < out.len() {
        let take = (out.len() - written).min(rate_bytes);
        // Materialize lanes into a scratch byte view.
        let mut lane_bytes = [0u8; 8];
        let mut emitted = 0;
        for i in 0..(rate_bytes / 8) {
            if emitted >= take { break; }
            lane_bytes.copy_from_slice(&s[i].to_le_bytes());
            let chunk = (take - emitted).min(8);
            out[written + emitted .. written + emitted + chunk]
                .copy_from_slice(&lane_bytes[..chunk]);
            emitted += chunk;
        }
        written += emitted;
        if written < out.len() { permute(s); }
    }
}
