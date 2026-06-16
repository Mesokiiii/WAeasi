//! X25519 — ECDH on Curve25519 (RFC 7748 § 5).
//!
//! Constant-time scalar multiplication via Montgomery ladder.  All
//! field arithmetic is over GF(2^255 - 19), represented as 5 limbs of
//! 51 bits (radix 2^51).
const P: u64 = (1 << 51) - 1;

type Fe = [u64; 5];

fn fe_zero() -> Fe { [0; 5] }
fn fe_one()  -> Fe { let mut f = fe_zero(); f[0] = 1; f }

fn fe_add(out: &mut Fe, a: &Fe, b: &Fe) { for i in 0..5 { out[i] = a[i] + b[i]; } }
fn fe_sub(out: &mut Fe, a: &Fe, b: &Fe) {
    let mask = (1u64 << 51) * 18;
    out[0] = a[0] + mask - b[0];
    for i in 1..5 { out[i] = a[i] + mask + (1 << 51) - b[i]; }
}

fn fe_carry(f: &mut Fe) {
    let c0 = f[0] >> 51; f[0] &= P; f[1] += c0;
    let c1 = f[1] >> 51; f[1] &= P; f[2] += c1;
    let c2 = f[2] >> 51; f[2] &= P; f[3] += c2;
    let c3 = f[3] >> 51; f[3] &= P; f[4] += c3;
    let c4 = f[4] >> 51; f[4] &= P; f[0] += c4 * 19;
}

fn fe_mul(out: &mut Fe, a: &Fe, b: &Fe) {
    let mut t = [0u128; 5];
    for i in 0..5 {
        for j in 0..5 {
            let bj = b[j] as u128 * if i + j > 4 { 19 } else { 1 };
            t[(i + j) % 5] += a[i] as u128 * bj;
        }
    }
    for i in 0..5 { out[i] = (t[i] & ((1u128 << 51) - 1)) as u64; }
    let mut k = 0u128;
    for i in 0..5 {
        k += t[i] >> 51;
        out[i] += k as u64 & P;
        k >>= 51;
    }
    out[0] += (k as u64) * 19;
    fe_carry(out);
}

#[inline] fn sq(a: &Fe) -> Fe { let mut o = fe_zero(); fe_mul(&mut o, a, a); o }
#[inline] fn mul(a: &Fe, b: &Fe) -> Fe { let mut o = fe_zero(); fe_mul(&mut o, a, b); o }

fn fe_invert(out: &mut Fe, z: &Fe) {
    // 2^255 - 21 exponentiation chain via the canonical 254-step ladder.
    let z2    = sq(z);
    let z9    = mul(z, &sq(&sq(&z2)));
    let z11   = mul(&z9, &z2);
    let mut x = mul(&sq(&z11), &z9);                   // 2^5 - 1
    for _ in 0..5  { x = sq(&x); }
    x = mul(&x, &z9);                                  // 2^10 - 1
    let mut y = x;
    for _ in 0..10 { y = sq(&y); }
    y = mul(&y, &x);                                   // 2^20 - 1
    let mut z2_50 = y;
    for _ in 0..20 { z2_50 = sq(&z2_50); }
    z2_50 = mul(&z2_50, &y);                           // 2^40 - 1
    for _ in 0..10 { z2_50 = sq(&z2_50); }
    z2_50 = mul(&z2_50, &x);                           // 2^50 - 1
    let mut z2_100 = z2_50;
    for _ in 0..50 { z2_100 = sq(&z2_100); }
    z2_100 = mul(&z2_100, &z2_50);                     // 2^100 - 1
    let mut z2_200 = z2_100;
    for _ in 0..100 { z2_200 = sq(&z2_200); }
    z2_200 = mul(&z2_200, &z2_100);
    for _ in 0..50  { z2_200 = sq(&z2_200); }
    z2_200 = mul(&z2_200, &z2_50);
    for _ in 0..5   { z2_200 = sq(&z2_200); }
    *out = mul(&z2_200, &z9);
}

fn cswap(swap: u64, a: &mut Fe, b: &mut Fe) {
    let mask = 0u64.wrapping_sub(swap);
    for i in 0..5 { let t = mask & (a[i] ^ b[i]); a[i] ^= t; b[i] ^= t; }
}

/// Montgomery-ladder X25519.  `scalar` and `u` are 32 little-endian bytes.
pub fn x25519(scalar: &[u8; 32], u: &[u8; 32]) -> [u8; 32] {
    let mut k = *scalar;
    k[0] &= 248; k[31] &= 127; k[31] |= 64;

    let mut x1 = unpack(u);
    let mut x2 = fe_one();
    let mut z2 = fe_zero();
    let mut x3 = x1;
    let mut z3 = fe_one();
    let mut swap: u64 = 0;

    for t in (0..255).rev() {
        let bit = ((k[t >> 3] >> (t & 7)) & 1) as u64;
        swap ^= bit;
        cswap(swap, &mut x2, &mut x3);
        cswap(swap, &mut z2, &mut z3);
        swap = bit;
        ladder_step(&mut x1, &mut x2, &mut z2, &mut x3, &mut z3);
    }
    cswap(swap, &mut x2, &mut x3);
    cswap(swap, &mut z2, &mut z3);

    let mut zinv = fe_zero();
    fe_invert(&mut zinv, &z2);
    let mut out_fe = fe_zero();
    fe_mul(&mut out_fe, &x2, &zinv);
    pack(&out_fe)
}

fn unpack(bytes: &[u8; 32]) -> Fe {
    let mut f = fe_zero();
    let load = |s: &[u8]| u64::from_le_bytes(s.try_into().unwrap());
    f[0] =   load(&bytes[0..8])               & P;
    f[1] = ((load(&bytes[6..14])  >>  3))     & P;
    f[2] = ((load(&bytes[12..20]) >>  6))     & P;
    f[3] = ((load(&bytes[19..27]) >>  1))     & P;
    f[4] = ((load(&bytes[24..32]) >> 12))     & P;
    f
}

fn pack(f: &Fe) -> [u8; 32] {
    let mut h = *f;
    fe_carry(&mut h);
    let mut out = [0u8; 32];
    let combined = h[0] | (h[1] << 51);
    out[..8].copy_from_slice(&combined.to_le_bytes());
    let combined = (h[1] >> 13) | (h[2] << 38);
    out[8..16].copy_from_slice(&combined.to_le_bytes());
    let combined = (h[2] >> 26) | (h[3] << 25);
    out[16..24].copy_from_slice(&combined.to_le_bytes());
    let combined = (h[3] >> 39) | (h[4] << 12);
    out[24..32].copy_from_slice(&combined.to_le_bytes());
    out
}

fn ladder_step(x1: &mut Fe, x2: &mut Fe, z2: &mut Fe, x3: &mut Fe, z3: &mut Fe) {
    let mut a = fe_zero(); fe_add(&mut a, x2, z2);
    let aa = sq(&a);
    let mut b = fe_zero(); fe_sub(&mut b, x2, z2);
    let bb = sq(&b);
    let mut e = fe_zero(); fe_sub(&mut e, &aa, &bb);
    let mut c = fe_zero(); fe_add(&mut c, x3, z3);
    let mut d = fe_zero(); fe_sub(&mut d, x3, z3);
    let da = mul(&d, &a);
    let cb = mul(&c, &b);
    let mut sum = fe_zero(); fe_add(&mut sum, &da, &cb);
    let new_x3 = sq(&sum);
    let mut diff = fe_zero(); fe_sub(&mut diff, &da, &cb);
    let diff_sq = sq(&diff);
    let new_z3 = mul(x1, &diff_sq);
    let new_x2 = mul(&aa, &bb);
    let mut e_a24 = fe_zero(); e_a24[0] = 121665u64;
    let tmp = mul(&e, &e_a24);
    let mut sum2 = fe_zero(); fe_add(&mut sum2, &aa, &tmp);
    let new_z2 = mul(&e, &sum2);

    *x2 = new_x2; *z2 = new_z2; *x3 = new_x3; *z3 = new_z3;
}

/// Generate an X25519 public key from a private scalar.
pub fn public_key(secret: &[u8; 32]) -> [u8; 32] {
    let mut basepoint = [0u8; 32]; basepoint[0] = 9;
    x25519(secret, &basepoint)
}
