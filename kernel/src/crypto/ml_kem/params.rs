//! FIPS 203 parameter sets — Table 2.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MlKemLevel { K512, K768, K1024 }

#[derive(Copy, Clone, Debug)]
pub struct Params {
    pub level:        MlKemLevel,
    /// `n` — degree of polynomials (always 256).
    pub n:            usize,
    /// `q` — modulus (always 3329).
    pub q:            u16,
    /// `k` — module rank.
    pub k:            usize,
    /// `eta1` — error sampling parameter for s, e.
    pub eta1:         u8,
    /// `eta2` — error sampling parameter for r, e1, e2.
    pub eta2:         u8,
    /// `du`, `dv` — compression bits.
    pub du:           u8,
    pub dv:           u8,

    /// Public-key byte length: `384 * k + 32`.
    pub pk_len:       usize,
    /// Secret-key byte length: `768 * k + 96`.
    pub sk_len:       usize,
    /// Ciphertext byte length: `32 * (du * k + dv)`.
    pub ct_len:       usize,
}

pub const MLKEM_512: Params = Params {
    level: MlKemLevel::K512,
    n: 256, q: 3329, k: 2, eta1: 3, eta2: 2, du: 10, dv: 4,
    pk_len:  800, sk_len: 1632, ct_len:  768,
};

pub const MLKEM_768: Params = Params {
    level: MlKemLevel::K768,
    n: 256, q: 3329, k: 3, eta1: 2, eta2: 2, du: 10, dv: 4,
    pk_len: 1184, sk_len: 2400, ct_len: 1088,
};

pub const MLKEM_1024: Params = Params {
    level: MlKemLevel::K1024,
    n: 256, q: 3329, k: 4, eta1: 2, eta2: 2, du: 11, dv: 5,
    pk_len: 1568, sk_len: 3168, ct_len: 1568,
};
