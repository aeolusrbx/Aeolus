//! Content overlay: replacing files inside a Roblox installation (custom sky,
//! sounds, cursors, fonts) safely and reversibly.
//!
//! * Only files that already exist in the installation are replaced.
//! * Roblox's original of every replaced file is saved once in
//!   `<install>/.aeolus-originals/` and put back by [`restore`].
//! * `<install>/.aeolus-overlay.json` records what is replaced, so Verify can
//!   skip those files and a later sync can undo them.

use crate::configuration::store;
use crate::error::AppResult;
use crate::paths::safe_join;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const OVERLAY_FILE: &str = ".aeolus-overlay.json";
pub const ORIGINALS_DIR: &str = ".aeolus-originals";

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Overlay {
    pub sky_pack_id: Option<String>,
    pub sky_pack_name: Option<String>,
    /// Mod slots currently applied.
    pub mods: Vec<String>,
    /// Paths (relative to the install) that currently hold Aeolus-provided files.
    pub files: Vec<String>,
    /// Identifies the exact set of sources applied, to skip redundant work.
    pub fingerprint: String,
}

/// One file to place into the installation.
#[derive(Debug, Clone)]
pub struct Planned {
    /// Path relative to the install, using `/`.
    pub rel: String,
    pub source: PathBuf,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub written: usize,
    /// Target paths that don't exist in this Roblox version (left untouched).
    pub skipped: Vec<String>,
    /// Nothing changed because the same set was already applied.
    pub unchanged: bool,
}

pub fn read_overlay(install: &Path) -> Overlay {
    std::fs::read(install.join(OVERLAY_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

/// Put back every original Aeolus replaced and forget the overlay.
pub fn restore(install: &Path) -> AppResult<usize> {
    let ov = read_overlay(install);
    let mut n = 0;
    for rel in &ov.files {
        let (Some(target), Some(orig)) = (safe_join(install, rel), safe_join(&install.join(ORIGINALS_DIR), rel)) else { continue };
        if orig.is_file() {
            std::fs::copy(&orig, &target)?;
            n += 1;
        }
    }
    let _ = std::fs::remove_dir_all(install.join(ORIGINALS_DIR));
    let _ = std::fs::remove_file(install.join(OVERLAY_FILE));
    Ok(n)
}

fn fingerprint(planned: &[Planned]) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    for p in planned {
        let meta = std::fs::metadata(&p.source).ok();
        let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
        let mtime = meta
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        h.update(format!("{}|{}|{size}|{mtime}\n", p.rel, p.source.display()).as_bytes());
    }
    hex::encode(h.finalize())
}

/// Make the installation contain exactly `planned` (on top of Roblox's originals).
pub fn sync(install: &Path, planned: &[Planned], sky: Option<(&str, &str)>, mods: &[String]) -> AppResult<SyncReport> {
    let fp = fingerprint(planned);
    let current = read_overlay(install);
    if current.fingerprint == fp && current.files.iter().all(|f| install.join(f).exists()) && (!planned.is_empty() || current.files.is_empty()) {
        return Ok(SyncReport { unchanged: true, ..Default::default() });
    }
    restore(install)?;
    let mut report = SyncReport::default();
    let mut files = vec![];
    for p in planned {
        let Some(target) = safe_join(install, &p.rel) else { continue };
        if !target.is_file() {
            report.skipped.push(p.rel.clone());
            continue;
        }
        if !p.source.is_file() {
            report.skipped.push(p.rel.clone());
            continue;
        }
        let orig = install.join(ORIGINALS_DIR).join(&p.rel);
        if let Some(parent) = orig.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !orig.exists() {
            std::fs::copy(&target, &orig)?;
        }
        std::fs::copy(&p.source, &target)?;
        if !files.contains(&p.rel) {
            files.push(p.rel.clone());
        }
    }
    report.written = files.len();
    if files.is_empty() {
        let _ = std::fs::remove_dir_all(install.join(ORIGINALS_DIR));
        return Ok(report);
    }
    store::write_json(
        &install.join(OVERLAY_FILE),
        &Overlay {
            sky_pack_id: sky.map(|(id, _)| id.to_string()),
            sky_pack_name: sky.map(|(_, n)| n.to_string()),
            mods: mods.to_vec(),
            files,
            fingerprint: fp,
        },
    )?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_replaces_restores_and_skips_unknown() {
        let d = tempfile::tempdir().unwrap();
        let inst = d.path().join("inst");
        std::fs::create_dir_all(inst.join("content/sounds")).unwrap();
        std::fs::write(inst.join("content/sounds/ouch.ogg"), b"orig").unwrap();
        let src = d.path().join("mine.ogg");
        std::fs::write(&src, b"mine").unwrap();
        let plan = vec![
            Planned { rel: "content/sounds/ouch.ogg".into(), source: src.clone() },
            Planned { rel: "content/sounds/nope.ogg".into(), source: src.clone() },
        ];
        let r = sync(&inst, &plan, None, &["death".into()]).unwrap();
        assert_eq!(r.written, 1);
        assert_eq!(r.skipped, vec!["content/sounds/nope.ogg".to_string()]);
        assert!(!inst.join("content/sounds/nope.ogg").exists());
        assert_eq!(std::fs::read(inst.join("content/sounds/ouch.ogg")).unwrap(), b"mine");
        assert!(sync(&inst, &plan, None, &["death".into()]).unwrap().unchanged);
        // emptying the plan restores the original
        sync(&inst, &[], None, &[]).unwrap();
        assert_eq!(std::fs::read(inst.join("content/sounds/ouch.ogg")).unwrap(), b"orig");
        assert!(!inst.join(OVERLAY_FILE).exists());
    }
}
