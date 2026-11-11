//! Aeolus self-updater.
//!
//! Reads a release feed (JSON over HTTPS) configured in Settings > Advanced:
//! `{ "version": "0.2.0", "url": "https://.../Aeolus_0.2.0_x64-setup.exe",
//!    "sha256": "<hex>", "notes": "..." }`
//! The installer is downloaded to the local data folder, its SHA-256 verified,
//! and then launched after Aeolus exits, so no file is replaced while in use.

use crate::error::{AppError, AppResult};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    pub version: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfUpdateCheck {
    pub current: String,
    pub latest: Option<ReleaseInfo>,
    pub update_available: bool,
    pub configured: bool,
}

pub fn newer(candidate: &str, current: &str) -> bool {
    let p = |v: &str| -> Vec<u64> { v.trim_start_matches('v').split(['.', '-']).take(3).map(|x| x.parse().unwrap_or(0)).collect() };
    p(candidate) > p(current)
}

pub fn validate_release(r: &ReleaseInfo) -> AppResult<()> {
    if !r.url.starts_with("https://") {
        return Err(AppError::Validation("Release URL must use https".into()));
    }
    if r.sha256.len() != 64 || !r.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::Validation("Release feed has an invalid SHA-256".into()));
    }
    if !r.url.to_ascii_lowercase().ends_with(".exe") && !r.url.to_ascii_lowercase().ends_with(".msi") {
        return Err(AppError::Validation("Release URL must point to an .exe or .msi installer".into()));
    }
    Ok(())
}

pub async fn check(http: &reqwest::Client, feed: &str, current: &str) -> AppResult<SelfUpdateCheck> {
    if feed.trim().is_empty() {
        return Ok(SelfUpdateCheck { current: current.into(), latest: None, update_available: false, configured: false });
    }
    let resp = http.get(feed.trim()).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AppError::Validation(
            "No release feed was found at that address. Check the URL, and that the repository and its latest release are public.".into(),
        ));
    }
    let r: ReleaseInfo = resp.error_for_status()?.json().await?;
    validate_release(&r)?;
    Ok(SelfUpdateCheck { current: current.into(), update_available: newer(&r.version, current), latest: Some(r), configured: true })
}

pub async fn download(http: &reqwest::Client, r: &ReleaseInfo, dir: &Path, progress: impl Fn(u64, Option<u64>)) -> AppResult<PathBuf> {
    validate_release(r)?;
    std::fs::create_dir_all(dir)?;
    let ext = if r.url.to_ascii_lowercase().ends_with(".msi") { "msi" } else { "exe" };
    let dest = dir.join(format!("Aeolus-{}-setup.{ext}", r.version.replace(['/', '\\', ':'], "_")));
    let part = dest.with_extension(format!("{ext}.part"));
    let resp = http.get(&r.url).send().await?.error_for_status()?;
    let total = resp.content_length();
    let mut f = std::fs::File::create(&part)?;
    let mut got = 0u64;
    let mut s = resp.bytes_stream();
    while let Some(chunk) = s.next().await {
        let c = chunk?;
        f.write_all(&c)?;
        got += c.len() as u64;
        progress(got, total);
    }
    f.sync_all()?;
    drop(f);
    let actual = crate::installation::deploy::sha256_file(&part)?;
    if !actual.eq_ignore_ascii_case(&r.sha256) {
        let _ = std::fs::remove_file(&part);
        return Err(AppError::Integrity("the downloaded Aeolus installer does not match the published checksum".into()));
    }
    std::fs::rename(&part, &dest)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_compare_and_validation() {
        assert!(newer("0.2.0", "0.1.9"));
        assert!(newer("v1.0", "0.9.9"));
        assert!(!newer("0.1.0", "0.1.0"));
        let mut r = ReleaseInfo { version: "1".into(), url: "https://x/a.exe".into(), sha256: "a".repeat(64), notes: String::new() };
        assert!(validate_release(&r).is_ok());
        r.url = "http://x/a.exe".into();
        assert!(validate_release(&r).is_err());
        r.url = "https://x/a.zip".into();
        assert!(validate_release(&r).is_err());
    }
}
