//! GF(2^255 - 19) field arithmetic for Ed25519.
//!
//! Same modulus as X25519, same 5×51-bit-limb representation.  Adds:
//!   * negation (Edwards needs subtraction along with addition),
//!   * conditional negation in constant time (for point compression).
const P: u64 = (1 << 51) - 1;

pub type Fe = [u64; 5];

pub fn zero() -> Fe { [0; 5] }
pub fn one()  -> Fe { let mut f = zero(); f[0] = 1; f }

pub fn add(a: &Fe, b: &Fe) -> Fe {
    let mut o = zero();
    for i in 0..5 { o[i] = a[i] + b[i]; }
    o
}

pub fn sub(a: &Fe, b: &Fe) -> Fe {
    let mask = (1u64 << 51) * 18;
    let mut o = zero();
    o[0] = a[0] + mask - b[0];
    for i in 1..5 { o[i] = a[i] + mask + (1 << 51) - b[i]; }
    o
}

pub fn neg(a: &Fe) -> Fe { sub(&zero(), a) }

pub fn carry(f: &mut Fe) {
    let c0 = f[0] >> 51; f[0] &= P; f[1] += c0;
    let c1 = f[1] >> 51; f[1] &= P; f[2] += c1;
    let c2 = f[2] >> 51; f[2] &= P; f[3] += c2;
    let c3 = f[3] >> 51; f[3] &= P; f[4] += c3;
    let c4 = f[4] >> 51; f[4] &= P; f[0] += c4 * 19;
}

pub fn mul(a: &Fe, b: &Fe) -> Fe {
    let mut t = [0u128; 5];
    for i in 0..5 {
        for j in 0..5 {
            let bj = b[j] as u128 * if i + j > 4 { 19 } else { 1 };
            t[(i + j) % 5] += a[i] as u128 * bj;
        }
    }
    let mut o = zero();
    for i in 0..5 { o[i] = (t[i] & ((1u128 << 51) - 1)) as u64; }
    let mut k = 0u128;
    for i in 0..5 {
        k += t[i] >> 51;
        o[i] += k as u64 & P;
        k >>= 51;
    }
    o[0] += (k as u64) * 19;
    carry(&mut o);
    o
}

pub fn sq(a: &Fe) -> Fe { mul(a, a) }

pub fn cswap(swap: u64, a: &mut Fe, b: &mut Fe) {
    let mask = 0u64.wrapping_sub(swap & 1);
    for i in 0..5 { let t = mask & (a[i] ^ b[i]); a[i] ^= t; b[i] ^= t; }
}

/// Pack a fully-reduced field element into 32 LE bytes.
pub fn pack(f: &Fe) -> [u8; 32] {
    let mut h = *f;
    carry(&mut h);
    let mut out = [0u8; 32];
    out[..8].copy_from_slice(&(h[0] | (h[1] << 51)).to_le_bytes());
    out[8..16].copy_from_slice(&((h[1] >> 13) | (h[2] << 38)).to_le_bytes());
    out[16..24].copy_from_slice(&((h[2] >> 26) | (h[3] << 25)).to_le_bytes());
    out[24..32].copy_from_slice(&((h[3] >> 39) | (h[4] << 12)).to_le_bytes());
    out
}

/// Unpack 32 LE bytes into a field element.
pub fn unpack(bytes: &[u8; 32]) -> Fe {
    let load = |s: &[u8]| u64::from_le_bytes(s.try_into().unwrap());
    let mut f = zero();
    f[0] =   load(&bytes[0..8])               & P;
    f[1] =  (load(&bytes[6..14])  >>  3)      & P;
    f[2] =  (load(&bytes[12..20]) >>  6)      & P;
    f[3] =  (load(&bytes[19..27]) >>  1)      & P;
    f[4] =  (load(&bytes[24..32]) >> 12)      & P;
    f
}

/// Inversion via Fermat's little theorem: `a^(p-2)` mod p.
pub fn invert(z: &Fe) -> Fe {
    let t0 = sq(z);
    let t1a = sq(&sq(&t0));
    let t1b = mul(z, &t1a);                                // z^9
    let z9 = t1b;
    let t0c = mul(&t1b, &t0);                              // z^11
    let t1d = sq(&t0c);
    let mut x = mul(&t1d, &z9);                            // 2^5 - 1
    for _ in 0..5  { let p = x; x = sq(&p); }
    let xp = x; x = mul(&xp, &z9);
    let mut y = x;
    for _ in 0..10 { let p = y; y = sq(&p); }
    let yp = y; y = mul(&yp, &x);
    let mut z50 = y;
    for _ in 0..20 { let p = z50; z50 = sq(&p); }
    let zp = z50; z50 = mul(&zp, &y);
    for _ in 0..10 { let p = z50; z50 = sq(&p); }
    let zp = z50; z50 = mul(&zp, &x);
    let mut z100 = z50;
    for _ in 0..50 { let p = z100; z100 = sq(&p); }
    let zp = z100; z100 = mul(&zp, &z50);
    let mut z200 = z100;
    for _ in 0..100 { let p = z200; z200 = sq(&p); }
    let zp = z200; z200 = mul(&zp, &z100);
    for _ in 0..50  { let p = z200; z200 = sq(&p); }
    let zp = z200; z200 = mul(&zp, &z50);
    for _ in 0..5   { let p = z200; z200 = sq(&p); }
    mul(&z200, &z9)
}
