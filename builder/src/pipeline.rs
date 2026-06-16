//! Pipeline orchestrator — wires every stage together.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::error::{BuildError, Result};
use crate::fs;
use crate::manifest::Manifest;
use crate::stages::{aot, bundle, compose, componentize, sign, wizer};

pub use crate::stages::componentize::Language;
pub use crate::stages::sign::KeySource;

#[derive(Debug, Clone)]
pub struct PipelineOptions {
    pub out_dir:     PathBuf,
    pub wit_path:    Option<PathBuf>,
    pub world:       String,
    pub language:    Language,
    pub working_dir: Option<PathBuf>,
    pub skip_wizer:  bool,
    pub split_engine: bool,
    pub aot:         bool,
    pub aot_target:  Option<String>,
    pub sdk_label:   String,        // "typescript@0.1.0" / "go@0.1.0" etc.
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            out_dir:      PathBuf::from("dist"),
            wit_path:     None,
            world:        "handler".into(),
            language:     Language::Auto,
            working_dir:  None,
            skip_wizer:   false,
            split_engine: false,
            aot:          false,
            aot_target:   None,
            sdk_label:    format!("builder@{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StageTiming {
    pub stage: &'static str,
    pub ms:    u128,
    pub bytes: Option<u64>,
}

#[derive(Debug)]
pub struct Report {
    pub bundle_path:     PathBuf,
    pub bundle_digest:   String,
    pub public_key_hex:  String,
    pub timings:         Vec<StageTiming>,
    pub total_ms:        u128,
}

pub struct Pipeline { opts: PipelineOptions }

impl Pipeline {
    pub fn new(opts: PipelineOptions) -> Result<Self> {
        fs::mkdir_p(&opts.out_dir)?;
        Ok(Self { opts })
    }

    pub fn run(
        &self,
        source: &Path,
        manifest: &Manifest,
        key: &KeySource,
    ) -> Result<Report> {
        let total_start = Instant::now();
        let mut timings = Vec::with_capacity(7);

        // 1. componentize / pass-through
        let raw = self.opts.out_dir.join("raw.wasm");
        let comp = componentize::run(&componentize::Input {
            source,
            out_path: &raw,
            wit_path: self.opts.wit_path.as_deref(),
            world: &self.opts.world,
            language: self.opts.language,
            working_dir: self.opts.working_dir.as_deref(),
        })?;
        timings.push(t("componentize", comp.duration_ms, Some(comp.size_bytes)));

        // 2. wizer
        let wizered = self.opts.out_dir.join("wizered.wasm");
        let mut wi = wizer::Input::new(&raw, &wizered);
        wi.skip = self.opts.skip_wizer;
        wi.optional = self.opts.skip_wizer;
        let w = wizer::run(&wi)?;
        timings.push(t("wizer", w.duration_ms, Some(w.size_bytes)));

        // 3. compose / split
        let cmp = compose::run(&compose::Input {
            component: &wizered,
            out_dir:   &self.opts.out_dir,
            split:     self.opts.split_engine,
        })?;
        timings.push(t("compose", cmp.duration_ms, Some(cmp.size_user_bytes)));

        // 4. AOT (optional)
        let aot_out = aot::run(&aot::Input {
            component:    &cmp.user_path,
            out_dir:      &self.opts.out_dir,
            user_digest:  &cmp.user_digest,
            enabled:      self.opts.aot,
            optional:     true,
            target_triple: self.opts.aot_target.as_deref(),
        })?;
        timings.push(t("aot", aot_out.duration_ms, Some(aot_out.size_bytes)));

        // 5. manifest
        let manifest_path = self.opts.out_dir.join("manifest.toml");
        let manifest_text = manifest.render(
            &cmp.user_digest,
            cmp.engine_digest.as_deref(),
            &self.opts.sdk_label,
        );
        fs::write_atomic(&manifest_path, manifest_text.as_bytes())?;
        timings.push(t("manifest", 0, Some(manifest_text.len() as u64)));

        // 6. sign
        let sgn = sign::run(&sign::Input {
            out_dir:       &self.opts.out_dir,
            user_digest:   &cmp.user_digest,
            engine_digest: cmp.engine_digest.as_deref(),
            manifest_path: &manifest_path,
            key,
        })?;
        timings.push(t("sign", sgn.duration_ms, None));

        // 7. assemble
        let bundle_path = self.opts.out_dir
            .join(format!("{}.waeasi-bundle", manifest.spec.name));
        let manifest_entry = bundle::BundleEntry { name: "manifest.toml", path: &manifest_path };
        let user_entry     = bundle::BundleEntry { name: "user.cwasm",    path: &cmp.user_path };
        let sig_entry      = bundle::BundleEntry { name: "signature.ed25519", path: &sgn.signature_path };

        let mut entries = vec![manifest_entry, user_entry, sig_entry];
        if let Some(eng) = &cmp.engine_path {
            entries.push(bundle::BundleEntry { name: "engine.cwasm", path: eng });
        }
        if let Some(aot_p) = &aot_out.aot_path {
            entries.push(bundle::BundleEntry { name: "user.cwasm.aot", path: aot_p });
        }

        let asm = bundle::assemble(&entries, &bundle_path)?;
        timings.push(t("assemble", 0, Some(asm.size_bytes)));

        Ok(Report {
            bundle_path:    asm.path,
            bundle_digest:  asm.digest_hex,
            public_key_hex: sgn.public_key_hex,
            timings,
            total_ms:       total_start.elapsed().as_millis(),
        })
    }
}

fn t(stage: &'static str, ms: u128, bytes: Option<u64>) -> StageTiming {
    StageTiming { stage, ms, bytes }
}

/// Pretty formatter for CLI output.
pub fn format_report(r: &Report) -> String {
    let fmt_b = |b: Option<u64>| -> String {
        match b {
            None      => "       -".into(),
            Some(n) if n < 1024              => format!("{n:>7} B"),
            Some(n) if n < 1024 * 1024       => format!("{:>6.1} KiB", n as f64 / 1024.0),
            Some(n)                          => format!("{:>6.2} MiB", n as f64 / (1024.0 * 1024.0)),
        }
    };
    let mut out = String::new();
    out.push_str(&format!("built {}\n", r.bundle_path.display()));
    out.push_str(&format!("digest sha256:{}…\n", &r.bundle_digest[..16]));
    out.push_str(&format!("key    {}…\n", &r.public_key_hex[..16]));
    for t in &r.timings {
        out.push_str(&format!(
            "  {:<13}  {:>5} ms  {}\n",
            t.stage, t.ms, fmt_b(t.bytes),
        ));
    }
    out.push_str("  ─────────────────────────────────────────\n");
    out.push_str(&format!("  total          {:>5} ms\n", r.total_ms));
    out
}

/// Convenience parser for KeySource strings used by the CLI:
///   `env:VAR` | `file:/path` | `raw:hex...`
pub fn parse_key_source(s: &str) -> Result<KeySource> {
    if let Some(rest) = s.strip_prefix("env:")  { return Ok(KeySource::Env(rest.into())); }
    if let Some(rest) = s.strip_prefix("file:") { return Ok(KeySource::File(rest.into())); }
    if let Some(rest) = s.strip_prefix("raw:")  {
        return Ok(KeySource::Raw(crate::digest::hex_decode(rest)?));
    }
    Err(BuildError::Signature(format!("unknown key source: {s}")))
}
