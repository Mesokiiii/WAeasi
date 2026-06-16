//! compose stage — engine/user split.
//!
//! When `split == false` (default) we emit a single user.cwasm and
//! report digests.  When `split == true` we shell out to `wasm-tools
//! component split`; if the tool is missing we degrade gracefully to
//! a single artefact and warn at the call-site.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::digest;
use crate::error::Result;
use crate::fs;

#[derive(Debug, Clone)]
pub struct Input<'a> {
    pub component: &'a Path,
    pub out_dir:   &'a Path,
    pub split:     bool,
}

#[derive(Debug)]
pub struct Output {
    pub user_path:         PathBuf,
    pub user_digest:       String,
    pub size_user_bytes:   u64,
    pub engine_path:       Option<PathBuf>,
    pub engine_digest:     Option<String>,
    pub size_engine_bytes: u64,
    pub duration_ms:       u128,
    pub split_attempted:   bool,
    pub split_succeeded:   bool,
}

pub fn run(input: &Input<'_>) -> Result<Output> {
    let start = Instant::now();
    fs::mkdir_p(input.out_dir)?;

    let user_path = input.out_dir.join("user.cwasm");
    let mut engine_path: Option<PathBuf> = None;
    let mut engine_size = 0u64;
    let mut engine_digest: Option<String> = None;
    let mut split_succeeded = false;

    if input.split {
        let wt = std::env::var_os("WAEASI_WASM_TOOLS")
            .unwrap_or_else(|| "wasm-tools".into());
        let engine_tmp = input.out_dir.join("engine.cwasm");

        let res = Command::new(&wt)
            .args(["component", "split"])
            .arg(input.component)
            .arg("--engine").arg(&engine_tmp)
            .arg("--user").arg(&user_path)
            .output();

        match res {
            Ok(o) if o.status.success() => {
                engine_size = fs::size(&engine_tmp)?;
                engine_digest = Some(digest::file(&engine_tmp)?);
                engine_path = Some(engine_tmp);
                split_succeeded = true;
            }
            _ => {
                // graceful degradation: caller may emit a warning.
                fs::copy(input.component, &user_path)?;
            }
        }
    } else {
        fs::copy(input.component, &user_path)?;
    }

    let user_digest = digest::file(&user_path)?;
    let size_user_bytes = fs::size(&user_path)?;

    Ok(Output {
        user_path,
        user_digest,
        size_user_bytes,
        engine_path,
        engine_digest,
        size_engine_bytes: engine_size,
        duration_ms: start.elapsed().as_millis(),
        split_attempted: input.split,
        split_succeeded,
    })
}
