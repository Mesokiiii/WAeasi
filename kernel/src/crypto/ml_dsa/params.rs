//! FIPS 204 parameter sets for ML-DSA.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MlDsaLevel { L44, L65, L87 }

pub const MLDSA_44: MlDsaLevel = MlDsaLevel::L44;
pub const MLDSA_65: MlDsaLevel = MlDsaLevel::L65;
pub const MLDSA_87: MlDsaLevel = MlDsaLevel::L87;

#[derive(Copy, Clone, Debug)]
pub struct Params {
    pub level:   MlDsaLevel,
    pub k:       u8,        // matrix rows
    pub l:       u8,        // matrix cols
    pub eta:     u8,        // private key entry bound
    pub tau:     u8,        // challenge weight
    pub beta:    u32,
    pub gamma1:  u32,
    pub gamma2:  u32,
    pub omega:   u8,
    pub pk_len:  usize,
    pub sk_len:  usize,
    pub sig_len: usize,
}

const P_44: Params = Params {
    level: MlDsaLevel::L44, k: 4, l: 4, eta: 2, tau: 39,
    beta: 78, gamma1: 1 << 17, gamma2: 95_232, omega: 80,
    pk_len: 1312, sk_len: 2560, sig_len: 2420,
};

const P_65: Params = Params {
    level: MlDsaLevel::L65, k: 6, l: 5, eta: 4, tau: 49,
    beta: 196, gamma1: 1 << 19, gamma2: 261_888, omega: 55,
    pk_len: 1952, sk_len: 4032, sig_len: 3309,
};

const P_87: Params = Params {
    level: MlDsaLevel::L87, k: 8, l: 7, eta: 2, tau: 60,
    beta: 120, gamma1: 1 << 19, gamma2: 261_888, omega: 75,
    pk_len: 2592, sk_len: 4896, sig_len: 4627,
};

pub fn for_level(level: MlDsaLevel) -> Params {
    match level {
        MlDsaLevel::L44 => P_44,
        MlDsaLevel::L65 => P_65,
        MlDsaLevel::L87 => P_87,
    }
}
