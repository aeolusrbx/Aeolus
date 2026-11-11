//! Downloading, verifying and laying out Roblox packages.

use super::manifest::{self, Package};
use super::progress::{eta, CancelToken, Meter, Progress, Sink};
use crate::configuration::store;
use crate::error::{AppError, AppResult};
use crate::paths::safe_join;
use futures_util::StreamExt;
use md5::{Digest as _, Md5};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const FILE_INDEX: &str = "aeolus-files.json";
pub const STAGING_SUFFIX: &str = ".aeolus-staging";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileRecord {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileIndex {
    pub schema_version: u32,
    pub version: String,
    pub version_guid: String,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub packages: Vec<Package>,
    pub files: Vec<FileRecord>,
}

pub fn md5_file(path: &Path) -> AppResult<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Md5::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

pub fn sha256_file(path: &Path) -> AppResult<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(hex::encode(h.finalize()))
}

pub struct Downloader<'a> {
    pub http: &'a reqwest::Client,
    pub hosts: Vec<String>,
    pub cache_dir: PathBuf,
    pub cancel: CancelToken,
    pub sink: Sink,
    pub operation: String,
}

impl Downloader<'_> {
    async fn get_text(&self, path: &str) -> AppResult<String> {
        let mut last = None;
        for host in &self.hosts {
            self.cancel.check()?;
            match self.http.get(format!("{host}/{path}")).send().await.and_then(|r| r.error_for_status()) {
                Ok(r) => return Ok(r.text().await?),
                Err(e) => last = Some(e),
            }
        }
        Err(last.map(AppError::from).unwrap_or_else(|| AppError::Network("no deployment hosts configured".into())))
    }

    pub async fn fetch_manifest(&self, guid: &str) -> AppResult<Vec<Package>> {
        if !manifest::is_valid_guid(guid) {
            return Err(AppError::Validation(format!("invalid version id '{guid}'")));
        }
        manifest::parse_package_manifest(&self.get_text(&format!("{guid}-rbxPkgManifest.txt")).await?)
    }

    fn cached_path(&self, p: &Package) -> PathBuf {
        self.cache_dir.join(format!("{}.zip", p.md5))
    }

    /// Download every package (resuming partial files, skipping verified cached ones).
    /// Returns package paths in the cache.
    pub async fn download_all(&self, guid: &str, packages: &[Package]) -> AppResult<Vec<(Package, PathBuf)>> {
        std::fs::create_dir_all(&self.cache_dir)?;
        let total: u64 = packages.iter().map(|p| p.packed_size).sum();
        let mut completed: u64 = 0;
        let mut transferred: u64 = 0;
        let mut meter = Meter::new();
        let mut out = vec![];
        let steps = packages.len() as u32;
        for (i, p) in packages.iter().enumerate() {
            self.cancel.check()?;
            let dest = self.cached_path(p);
            if dest.exists() {
                let ok = std::fs::metadata(&dest).map(|m| m.len() == p.packed_size).unwrap_or(false)
                    && md5_file(&dest).map(|h| h == p.md5).unwrap_or(false);
                if ok {
                    completed += p.packed_size;
                    out.push((p.clone(), dest));
                    continue;
                }
                let _ = std::fs::remove_file(&dest);
            }
            let mut attempt = 0;
            loop {
                attempt += 1;
                let res = self
                    .download_one(guid, p, &mut completed, &mut transferred, total, &mut meter, i as u32 + 1, steps)
                    .await;
                match res {
                    Ok(()) => break,
                    Err(AppError::Integrity(m)) if attempt < 3 => {
                        let _ = std::fs::remove_file(self.cache_dir.join(format!("{}.part", p.md5)));
                        (self.sink)(Progress {
                            operation: self.operation.clone(),
                            stage: "Downloading".into(),
                            detail: Some(format!("{} failed verification ({m}); retrying", p.name)),
                            downloaded: completed,
                            total: Some(total),
                            step: i as u32 + 1,
                            steps,
                            ..Default::default()
                        });
                    }
                    Err(e) => return Err(e),
                }
            }
            completed += p.packed_size;
            out.push((p.clone(), dest));
        }
        Ok(out)
    }

    #[allow(clippy::too_many_arguments)]
    async fn download_one(
        &self,
        guid: &str,
        p: &Package,
        completed: &mut u64,
        transferred: &mut u64,
        total: u64,
        meter: &mut Meter,
        step: u32,
        steps: u32,
    ) -> AppResult<()> {
        let part = self.cache_dir.join(format!("{}.part", p.md5));
        let mut last_err = AppError::Network("no deployment hosts configured".into());
        for host in &self.hosts {
            self.cancel.check()?;
            let mut start = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            if start > p.packed_size {
                std::fs::remove_file(&part)?;
                start = 0;
            }
            if start == p.packed_size && start > 0 {
                break; // already complete, verify below
            }
            let url = format!("{host}/{guid}-{}", p.name);
            let mut req = self.http.get(&url);
            if start > 0 {
                req = req.header(reqwest::header::RANGE, format!("bytes={start}-"));
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => {
                    last_err = e.into();
                    continue;
                }
            };
            let status = resp.status();
            let append = if status == reqwest::StatusCode::PARTIAL_CONTENT {
                true
            } else if status.is_success() {
                false
            } else {
                last_err = AppError::Network(format!("{} returned HTTP {status} for {}", host, p.name));
                continue;
            };
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .append(append)
                .truncate(!append)
                .open(&part)?;
            let mut have = if append { start } else { 0 };
            let mut stream = resp.bytes_stream();
            let mut failed = None;
            while let Some(chunk) = stream.next().await {
                if self.cancel.is_cancelled() {
                    file.flush()?;
                    return Err(AppError::Cancelled);
                }
                match chunk {
                    Ok(bytes) => {
                        file.write_all(&bytes)?;
                        have += bytes.len() as u64;
                        *transferred += bytes.len() as u64;
                        if have > p.packed_size {
                            failed = Some(AppError::Integrity(format!("{} is larger than expected", p.name)));
                            break;
                        }
                        meter.sample(*transferred);
                        if meter.should_emit() {
                            let done = *completed + have;
                            let speed = meter.speed();
                            (self.sink)(Progress {
                                operation: self.operation.clone(),
                                stage: "Downloading".into(),
                                detail: Some(p.name.clone()),
                                downloaded: done,
                                total: Some(total),
                                bytes_per_second: speed,
                                eta_seconds: eta(total.saturating_sub(done), speed),
                                step,
                                steps,
                                done: false,
                            });
                        }
                    }
                    Err(e) => {
                        failed = Some(e.into());
                        break;
                    }
                }
            }
            file.flush()?;
            drop(file);
            match failed {
                Some(e @ AppError::Integrity(_)) => return Err(e),
                Some(e) => {
                    last_err = e; // keep the partial file so the next host/attempt resumes
                    continue;
                }
                None => break,
            }
        }
        let len = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        if len != p.packed_size {
            return Err(if len == 0 { last_err } else { AppError::Network(format!("{} download incomplete ({len} of {} bytes); it will resume next time", p.name, p.packed_size)) });
        }
        let actual = md5_file(&part)?;
        if actual != p.md5 {
            return Err(AppError::Integrity(format!("{} checksum mismatch", p.name)));
        }
        std::fs::rename(&part, self.cached_path(p))?;
        Ok(())
    }
}

/// Extract packages into `staging`. Returns the file index records and warnings.
pub fn extract_all(
    staging: &Path,
    packages: &[(Package, PathBuf)],
    cancel: &CancelToken,
    on_package: &dyn Fn(usize, &str),
) -> AppResult<(Vec<FileRecord>, Vec<String>)> {
    std::fs::create_dir_all(staging)?;
    let mut records = vec![];
    let mut warnings = vec![];
    for (i, (pkg, path)) in packages.iter().enumerate() {
        cancel.check()?;
        if !pkg.name.to_ascii_lowercase().ends_with(".zip") {
            continue; // e.g. RobloxPlayerInstaller.exe - not part of the layout
        }
        let Some(sub) = manifest::package_directory(&pkg.name) else {
            warnings.push(format!("Unknown package '{}' was skipped; Aeolus may need an update for this Roblox version", pkg.name));
            continue;
        };
        on_package(i, &pkg.name);
        let base = safe_join(staging, sub).ok_or_else(|| AppError::Integrity("bad package directory".into()))?;
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        for idx in 0..zip.len() {
            cancel.check()?;
            let mut entry = zip.by_index(idx)?;
            let name = entry.name().replace('\\', "/");
            let target = safe_join(&base, &name)
                .ok_or_else(|| AppError::Integrity(format!("{} contains an unsafe path '{name}'", pkg.name)))?;
            if entry.is_dir() || name.ends_with('/') {
                std::fs::create_dir_all(&target)?;
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&target)?;
            let mut hasher = Sha256::new();
            let mut buf = vec![0u8; 128 * 1024];
            let mut size = 0u64;
            loop {
                let n = entry.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
                out.write_all(&buf[..n])?;
                size += n as u64;
            }
            let rel = target.strip_prefix(staging).unwrap_or(&target).to_string_lossy().replace('\\', "/");
            records.retain(|r: &FileRecord| r.path != rel);
            records.push(FileRecord { path: rel, size, sha256: hex::encode(hasher.finalize()) });
        }
    }
    std::fs::write(staging.join("AppSettings.xml"), manifest::APP_SETTINGS_XML)?;
    if !staging.join(manifest::PLAYER_EXE).exists() {
        return Err(AppError::Integrity(format!("{} is missing after extraction", manifest::PLAYER_EXE)));
    }
    Ok((records, warnings))
}

/// Move a completed staging folder into place atomically, replacing any existing copy.
pub fn finalize(root: &Path, guid: &str, staging: &Path, index: &FileIndex) -> AppResult<PathBuf> {
    store::write_json(&staging.join(FILE_INDEX), index)?;
    let final_dir = root.join(guid);
    if final_dir.exists() {
        let old = root.join(format!("{guid}.aeolus-old-{}", chrono::Local::now().format("%Y%m%d%H%M%S")));
        std::fs::rename(&final_dir, &old)?;
        if let Err(e) = std::fs::rename(staging, &final_dir) {
            let _ = std::fs::rename(&old, &final_dir); // roll back
            return Err(e.into());
        }
        let _ = std::fs::remove_dir_all(&old);
    } else {
        std::fs::rename(staging, &final_dir)?;
    }
    Ok(final_dir)
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VerifyReport {
    pub checked: usize,
    pub missing: Vec<String>,
    pub modified: Vec<String>,
    pub index_missing: bool,
    /// Files intentionally replaced by Aeolus (e.g. a custom sky); not counted as damage.
    pub overlaid: usize,
}
impl VerifyReport {
    pub fn ok(&self) -> bool {
        !self.index_missing && self.missing.is_empty() && self.modified.is_empty()
    }
}

/// Check an Aeolus-managed version folder against its file index.
pub fn verify(dir: &Path, full: bool, cancel: &CancelToken, progress: &dyn Fn(usize, usize)) -> AppResult<VerifyReport> {
    let mut r = VerifyReport::default();
    let idx: FileIndex = match std::fs::read(dir.join(FILE_INDEX)) {
        Ok(b) => serde_json::from_slice(&b).map_err(|e| AppError::Integrity(format!("file index unreadable: {e}")))?,
        Err(_) => {
            r.index_missing = true;
            if !dir.join(manifest::PLAYER_EXE).exists() {
                r.missing.push(manifest::PLAYER_EXE.into());
            }
            return Ok(r);
        }
    };
    let overlay = crate::overlay::read_overlay(dir).files;
    let n = idx.files.len();
    for (i, f) in idx.files.iter().enumerate() {
        cancel.check()?;
        if i % 50 == 0 {
            progress(i, n);
        }
        if overlay.iter().any(|o| o == &f.path) {
            r.overlaid += 1;
            r.checked += 1;
            continue;
        }
        let Some(p) = safe_join(dir, &f.path) else {
            r.modified.push(f.path.clone());
            continue;
        };
        match std::fs::metadata(&p) {
            Err(_) => r.missing.push(f.path.clone()),
            Ok(m) if m.len() != f.size => r.modified.push(f.path.clone()),
            Ok(_) if full => {
                if sha256_file(&p)? != f.sha256 {
                    r.modified.push(f.path.clone());
                }
            }
            Ok(_) => {}
        }
        r.checked += 1;
    }
    progress(n, n);
    Ok(r)
}
