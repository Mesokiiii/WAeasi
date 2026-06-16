//! sign stage — Ed25519 detached signature.
//!
//! Bundle digest is a SHA-256 over (version ‖ engine-digest ‖ user-
//! digest ‖ manifest bytes), matching the TS and Python SDKs byte-for-
//! byte so signatures are interchangeable across SDKs.
//!
//! Key sources (priority order):
//!   * `KeySource::Raw(seed)`     — 32-byte seed in memory
//!   * `KeySource::File(path)`    — PEM-wrapped seed
//!   * `KeySource::Env(var_name)` — hex-encoded seed in env
//!
//! Output is a packed binary blob:
//!
//!   "WAEASIv1" | u8 ver | u8 sig_len | sig | u8 pk_len | pk | digest

use std::path::{Path, PathBuf};
use std::time::Instant;

use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};

use crate::digest;
use crate::error::{BuildError, Result};
use crate::fs;

const MAGIC: &[u8; 8] = b"WAEASIv1";
const VERSION: u8 = 1;

#[derive(Debug, Clone)]
pub enum KeySource {
    Raw(Vec<u8>),
    File(PathBuf),
    Env(String),
}

#[derive(Debug, Clone)]
pub struct Input<'a> {
    pub out_dir:       &'a Path,
    pub user_digest:   &'a str,
    pub engine_digest: Option<&'a str>,
    pub manifest_path: &'a Path,
    pub key:           &'a KeySource,
}

#[derive(Debug)]
pub struct Output {
    pub signature_path:    PathBuf,
    pub bundle_digest_hex: String,
    pub public_key_hex:    String,
    pub duration_ms:       u128,
}

pub fn run(input: &Input<'_>) -> Result<Output> {
    let start = Instant::now();
    let seed = load_seed(input.key)?;
    if seed.len() != 32 {
        return Err(BuildError::Signature(
            format!("expected 32-byte ed25519 seed, got {}", seed.len()),
        ));
    }
    let mut seed_arr = [0u8; 32];
    seed_arr.copy_from_slice(&seed);

    let signing = SigningKey::from_bytes(&seed_arr);
    let verifying: VerifyingKey = signing.verifying_key();

    let manifest_bytes = fs::read(input.manifest_path)?;
    let bundle_digest = compute_bundle_digest(
        VERSION, input.engine_digest, input.user_digest, &manifest_bytes,
    )?;
    let sig = signing.sign(&bundle_digest);

    // Pack
    let pk = verifying.to_bytes();
    let sig_bytes = sig.to_bytes();
    let mut blob = Vec::with_capacity(
        MAGIC.len() + 1 + 1 + sig_bytes.len() + 1 + pk.len() + bundle_digest.len(),
    );
    blob.extend_from_slice(MAGIC);
    blob.push(VERSION);
    blob.push(sig_bytes.len() as u8);
    blob.extend_from_slice(&sig_bytes);
    blob.push(pk.len() as u8);
    blob.extend_from_slice(&pk);
    blob.extend_from_slice(&bundle_digest);

    fs::mkdir_p(input.out_dir)?;
    let sig_path = input.out_dir.join("signature.ed25519");
    fs::write_atomic(&sig_path, &blob)?;

    Ok(Output {
        signature_path:    sig_path,
        bundle_digest_hex: digest::hex_lower(&bundle_digest),
        public_key_hex:    digest::hex_lower(&pk),
        duration_ms:       start.elapsed().as_millis(),
    })
}

fn compute_bundle_digest(
    version: u8,
    engine_digest: Option<&str>,
    user_digest: &str,
    manifest_bytes: &[u8],
) -> Result<[u8; 32]> {
    let mut h = Sha256::new();
    h.update((version as u32).to_le_bytes());
    if let Some(eng) = engine_digest {
        let eb = digest::hex_decode(eng)?;
        h.update((eb.len() as u32).to_le_bytes());
        h.update(&eb);
    } else {
        h.update(0u32.to_le_bytes());
    }
    let ub = digest::hex_decode(user_digest)?;
    h.update((ub.len() as u32).to_le_bytes());
    h.update(&ub);
    h.update((manifest_bytes.len() as u32).to_le_bytes());
    h.update(manifest_bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize());
    Ok(out)
}

fn load_seed(src: &KeySource) -> Result<Vec<u8>> {
    match src {
        KeySource::Raw(s)   => Ok(s.clone()),
        KeySource::File(p)  => parse_pem_seed(&fs::read_text(p)?),
        KeySource::Env(var) => {
            let v = std::env::var(var).map_err(|_|
                BuildError::Signature(format!("env {var} unset")))?;
            digest::hex_decode(v.trim())
        }
    }
}

fn parse_pem_seed(text: &str) -> Result<Vec<u8>> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    let begin = "-----BEGIN WAEASI ED25519 SEED-----";
    let end   = "-----END WAEASI ED25519 SEED-----";
    let s = text.find(begin).ok_or_else(||
        BuildError::Signature("missing BEGIN marker".into()))?;
    let e = text.find(end).ok_or_else(||
        BuildError::Signature("missing END marker".into()))?;
    let inner: String = text[s + begin.len()..e]
        .chars().filter(|c| !c.is_whitespace()).collect();
    STANDARD.decode(inner.as_bytes()).map_err(|e|
        BuildError::Signature(format!("base64: {e}")))
}
