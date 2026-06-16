//! `CPUID` feature detection.
//!
//! Stage-10 expansion: hardware crypto and SIMD feature flags,
//! consumed by `crypto::aes::aesni`, `crypto::sha256` (SHA-NI), and
//! by stage-11 SIMD ChaCha20.
use core::arch::x86_64::__cpuid_count;
use spin::Once;

#[derive(Default, Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub rdrand:        bool,
    pub rdseed:        bool,
    pub nx:            bool,
    pub smep:          bool,
    pub smap:          bool,
    pub umip:          bool,
    pub pcid:          bool,
    pub invpcid:       bool,
    pub ibrs_ibpb:     bool,
    pub stibp:         bool,
    pub ssbd:          bool,
    pub fsgsbase:      bool,
    pub max_phys_bits: u8,
    pub max_virt_bits: u8,

    // ── Stage-10 hardware crypto / SIMD ──
    pub aes:           bool,    // AES-NI
    pub clmul:         bool,    // PCLMULQDQ
    pub sha:           bool,    // SHA-NI (sha256rnds2 / sha256msg{1,2})
    pub avx:           bool,
    pub avx2:          bool,
    pub avx512f:       bool,
    pub bmi1:          bool,
    pub bmi2:          bool,
    pub adx:           bool,    // ADCX/ADOX (big-int speedup)
    pub vaes:          bool,    // AVX-512 vector AES
    pub vpclmulqdq:    bool,    // AVX-512 vector CLMUL
}

static FEATURES: Once<CpuFeatures> = Once::new();

#[inline]
fn cpuid(leaf: u32, sub: u32) -> (u32, u32, u32, u32) {
    let r = unsafe { __cpuid_count(leaf, sub) };
    (r.eax, r.ebx, r.ecx, r.edx)
}

pub fn probe() -> &'static CpuFeatures {
    FEATURES.call_once(|| {
        let mut f = CpuFeatures::default();

        let (_, _, ecx1, _edx1) = cpuid(1, 0);
        f.rdrand = ecx1 & (1 << 30) != 0;
        f.pcid   = ecx1 & (1 << 17) != 0;
        f.aes    = ecx1 & (1 << 25) != 0;
        f.clmul  = ecx1 & (1 <<  1) != 0;
        f.avx    = ecx1 & (1 << 28) != 0;

        let (_, ebx7, ecx7, edx7) = cpuid(7, 0);
        f.rdseed     = ebx7 & (1 << 18) != 0;
        f.smep       = ebx7 & (1 <<  7) != 0;
        f.smap       = ebx7 & (1 << 20) != 0;
        f.umip       = ecx7 & (1 <<  2) != 0;
        f.invpcid    = ebx7 & (1 << 10) != 0;
        f.fsgsbase   = ebx7 & (1 <<  0) != 0;
        f.ibrs_ibpb  = edx7 & (1 << 26) != 0;
        f.stibp      = edx7 & (1 << 27) != 0;
        f.ssbd       = edx7 & (1 << 31) != 0;
        f.avx2       = ebx7 & (1 <<  5) != 0;
        f.avx512f    = ebx7 & (1 << 16) != 0;
        f.bmi1       = ebx7 & (1 <<  3) != 0;
        f.bmi2       = ebx7 & (1 <<  8) != 0;
        f.adx        = ebx7 & (1 << 19) != 0;
        f.sha        = ebx7 & (1 << 29) != 0;
        f.vaes       = ecx7 & (1 <<  9) != 0;
        f.vpclmulqdq = ecx7 & (1 << 10) != 0;

        let (_, _, _, edx_x1) = cpuid(0x8000_0001, 0);
        f.nx = edx_x1 & (1 << 20) != 0;

        let (eax_x8, _, _, _) = cpuid(0x8000_0008, 0);
        f.max_phys_bits = (eax_x8 & 0xFF) as u8;
        f.max_virt_bits = ((eax_x8 >> 8) & 0xFF) as u8;

        log::info!("[cpuid] phys={}b virt={}b nx={} smep={} smap={} rdrand={} rdseed={}",
                   f.max_phys_bits, f.max_virt_bits, f.nx, f.smep, f.smap,
                   f.rdrand, f.rdseed);
        log::info!("[cpuid] crypto: aes={} clmul={} sha={} vaes={} vpclmulqdq={}",
                   f.aes, f.clmul, f.sha, f.vaes, f.vpclmulqdq);
        log::info!("[cpuid] simd:   avx={} avx2={} avx512f={} bmi1={} bmi2={} adx={}",
                   f.avx, f.avx2, f.avx512f, f.bmi1, f.bmi2, f.adx);
        f
    })
}

#[inline]
pub fn features() -> &'static CpuFeatures {
    FEATURES.get().expect("cpuid::probe() not called")
}
