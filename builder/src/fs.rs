//! Filesystem helpers — keep stage code free of boilerplate.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::error::{BuildError, Result};

/// Idempotent `mkdir -p`.
pub fn mkdir_p(p: &Path) -> Result<()> {
    if p.exists() { return Ok(()); }
    fs::create_dir_all(p).map_err(|e| BuildError::io(p, e))
}

/// Read whole file into Vec<u8>.
pub fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| BuildError::io(path, e))
}

/// Read whole file as UTF-8 text.
pub fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| BuildError::io(path, e))
}

/// Atomic write: write to `path.tmp`, fsync, rename to `path`.
///
/// Used for any artefact whose partial state must never be observable
/// (manifest.toml, signature.ed25519, the final bundle).
pub fn write_atomic(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() { mkdir_p(parent)?; }
    let tmp = with_suffix(path, ".tmp");
    {
        let mut f = fs::OpenOptions::new()
            .create(true).write(true).truncate(true)
            .open(&tmp).map_err(|e| BuildError::io(&tmp, e))?;
        f.write_all(data).map_err(|e| BuildError::io(&tmp, e))?;
        sync(&mut f).map_err(|e| BuildError::io(&tmp, e))?;
    }
    fs::rename(&tmp, path).map_err(|e| BuildError::io(path, e))?;
    Ok(())
}

fn sync(f: &mut fs::File) -> io::Result<()> { f.sync_all() }

/// Ensure parent directory exists, then copy `src` to `dst` (overwriting).
pub fn copy(src: &Path, dst: &Path) -> Result<u64> {
    if let Some(parent) = dst.parent() { mkdir_p(parent)?; }
    fs::copy(src, dst).map_err(|e| BuildError::io(dst, e))
}

/// Append a suffix to a path, preserving the parent dir.
pub fn with_suffix(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// Get file size in bytes.
pub fn size(p: &Path) -> Result<u64> {
    Ok(fs::metadata(p).map_err(|e| BuildError::io(p, e))?.len())
}
