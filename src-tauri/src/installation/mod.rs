//! Installation management: Aeolus-managed Roblox Player installs.

pub mod deploy;
pub mod detect;
pub mod manifest;
pub mod progress;

use crate::error::{AppError, AppResult};
use deploy::{Downloader, FileIndex, STAGING_SUFFIX};
use manifest::ClientVersion;
use progress::{CancelToken, Progress, Sink};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledVersion {
    pub version: String,
    pub version_guid: String,
    pub path: String,
    pub installed_at: chrono::DateTime<chrono::Local>,
    pub file_count: usize,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct LatestCheck {
    pub checked_at: Option<chrono::DateTime<chrono::Local>>,
    pub latest: Option<ClientVersion>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct InstallState {
    pub schema_version: u32,
    /// The Aeolus-managed installation used for launching.
    pub active: Option<InstalledVersion>,
    /// If set, launch this detected (non-managed) installation instead.
    pub preferred_external: Option<String>,
    pub last_check: LatestCheck,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOutcome {
    pub installed: InstalledVersion,
    pub previous: Option<InstalledVersion>,
    pub warnings: Vec<String>,
    pub downloaded_bytes: u64,
}

pub struct InstallJob<'a> {
    pub http: &'a reqwest::Client,
    pub hosts: Vec<String>,
    pub root: PathBuf,
    pub cache_dir: PathBuf,
    pub keep_cache: bool,
    pub cancel: CancelToken,
    pub sink: Sink,
    pub operation: String,
}

fn emit(sink: &Sink, op: &str, stage: &str, detail: Option<String>, step: u32, steps: u32) {
    sink(Progress { operation: op.into(), stage: stage.into(), detail, step, steps, ..Default::default() });
}

pub async fn fetch_latest(http: &reqwest::Client, url: &str) -> AppResult<ClientVersion> {
    let body = http.get(url).send().await?.error_for_status()?.text().await?;
    manifest::parse_client_version(&body)
}

/// Download, verify and install `target`. On failure the previous installation
/// is left untouched; the staging folder and verified downloads are kept so the
/// next attempt resumes.
pub async fn install(job: &InstallJob<'_>, target: &ClientVersion, previous: Option<InstalledVersion>) -> AppResult<InstallOutcome> {
    std::fs::create_dir_all(&job.root).map_err(|e| {
        AppError::Io(std::io::Error::new(e.kind(), format!("cannot create installation folder {}: {e}", job.root.display())))
    })?;
    // Write test: catch read-only / permission problems before downloading anything.
    let probe = job.root.join(".aeolus-write-test");
    std::fs::write(&probe, b"ok").map_err(|e| {
        AppError::Io(std::io::Error::new(e.kind(), format!("the installation folder is not writable: {e}")))
    })?;
    let _ = std::fs::remove_file(probe);

    let op = job.operation.as_str();
    emit(&job.sink, op, "Fetching package list", Some(target.version.clone()), 0, 0);
    let dl = Downloader {
        http: job.http,
        hosts: job.hosts.clone(),
        cache_dir: job.cache_dir.clone(),
        cancel: job.cancel.clone(),
        sink: job.sink.clone(),
        operation: job.operation.clone(),
    };
    let packages = dl.fetch_manifest(&target.version_guid).await?;
    let downloaded = dl.download_all(&target.version_guid, &packages).await?;
    let total: u64 = packages.iter().map(|p| p.packed_size).sum();

    let staging = job.root.join(format!("{}{STAGING_SUFFIX}", target.version_guid));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?; // ours, from an interrupted run
    }
    let steps = downloaded.len() as u32;
    let sink = job.sink.clone();
    let cancel = job.cancel.clone();
    let opname = job.operation.clone();
    let staging2 = staging.clone();
    let dl2 = downloaded.clone();
    let (files, warnings) = tokio::task::spawn_blocking(move || {
        deploy::extract_all(&staging2, &dl2, &cancel, &|i, name| {
            sink(Progress {
                operation: opname.clone(),
                stage: "Extracting".into(),
                detail: Some(name.to_string()),
                downloaded: total,
                total: Some(total),
                step: i as u32 + 1,
                steps,
                ..Default::default()
            })
        })
    })
    .await
    .map_err(|e| AppError::Other(format!("extraction task failed: {e}")))??;

    job.cancel.check()?;
    emit(&job.sink, op, "Finalizing", None, steps, steps);
    let size = files.iter().map(|f| f.size).sum();
    let file_count = files.len();
    let index = FileIndex {
        schema_version: 1,
        version: target.version.clone(),
        version_guid: target.version_guid.clone(),
        created_at: chrono::Local::now(),
        packages: packages.clone(),
        files,
    };
    let final_dir = deploy::finalize(&job.root, &target.version_guid, &staging, &index)?;

    // Remove superseded Aeolus-managed versions (only folders carrying our index).
    if let Ok(rd) = std::fs::read_dir(&job.root) {
        for e in rd.flatten() {
            let p = e.path();
            let n = e.file_name().to_string_lossy().to_string();
            if p != final_dir && manifest::is_valid_guid(&n) && p.join(deploy::FILE_INDEX).exists() {
                let _ = std::fs::remove_dir_all(&p);
            }
        }
    }
    if !job.keep_cache {
        for (_, path) in &downloaded {
            let _ = std::fs::remove_file(path);
        }
    }
    let installed = InstalledVersion {
        version: target.version.clone(),
        version_guid: target.version_guid.clone(),
        path: final_dir.to_string_lossy().to_string(),
        installed_at: chrono::Local::now(),
        file_count,
        size,
    };
    job.sink.clone()(Progress { operation: job.operation.clone(), stage: "Completed".into(), done: true, downloaded: total, total: Some(total), ..Default::default() });
    Ok(InstallOutcome { installed, previous, warnings, downloaded_bytes: total })
}

/// Remove an Aeolus-managed version folder. Refuses to touch folders that Aeolus did not create.
pub fn remove_managed(dir: &Path) -> AppResult<()> {
    if !dir.join(deploy::FILE_INDEX).exists() && !dir.to_string_lossy().ends_with(STAGING_SUFFIX) {
        return Err(AppError::Validation(format!(
            "{} was not installed by Aeolus, so it will not be deleted",
            dir.display()
        )));
    }
    // Rename first so a half-deleted folder is never mistaken for a valid install.
    let doomed = dir.with_file_name(format!("{}.aeolus-deleting", dir.file_name().unwrap_or_default().to_string_lossy()));
    std::fs::rename(dir, &doomed)?;
    std::fs::remove_dir_all(&doomed)?;
    Ok(())
}

pub fn clear_cache(cache: &Path) -> AppResult<u64> {
    let mut freed = 0;
    if let Ok(rd) = std::fs::read_dir(cache) {
        for e in rd.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.ends_with(".zip") || n.ends_with(".part") {
                freed += e.metadata().map(|m| m.len()).unwrap_or(0);
                std::fs::remove_file(e.path())?;
            }
        }
    }
    Ok(freed)
}
