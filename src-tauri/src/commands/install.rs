//! Installation, update and launch commands.

use crate::diagnostics::activity::ActivityKind;
use crate::error::{AppError, AppResult};
use crate::installation::{self, deploy, detect, InstallOutcome, InstallState};
use crate::launcher::LaunchRecord;
use crate::services::{self, changed};
use crate::state::AppState;
use crate::updates::{UpdateRecord, UpdateStatus};
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub state: InstallState,
    pub update: UpdateStatus,
    pub detection: detect::DetectionResult,
    pub install_root: String,
    pub operation: Option<String>,
    pub launch_target: Option<String>,
}

fn detection(state: &AppState) -> detect::DetectionResult {
    let s = state.settings();
    let roots = detect::standard_roots(&state.managed_root(), s.installation.scan_official_install, s.installation.scan_other_bootstrappers);
    detect::scan(&roots)
}

#[tauri::command]
pub fn install_overview(state: State<'_, AppState>) -> Overview {
    let det = detection(&state);
    // Reconcile: if the recorded active install vanished, forget it (files are not ours to recreate).
    let mut st = state.install_state();
    if let Some(a) = &st.active {
        if !std::path::Path::new(&a.path).join(installation::manifest::PLAYER_EXE).exists() {
            state.logger.warn("install", format!("The Aeolus installation at {} is missing", a.path));
            let _ = state.update_install(|s| s.active = None);
            st = state.install_state();
        }
    }
    Overview {
        update: crate::updates::status(&st),
        launch_target: services::resolve_target(&state).ok().map(|t| t.dir.to_string_lossy().to_string()),
        state: st,
        detection: det,
        install_root: state.managed_root().to_string_lossy().into(),
        operation: state.current_operation(),
    }
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> AppResult<UpdateStatus> {
    services::check_updates(&app, true).await
}

/// kind: install | update | repair | reinstall
#[tauri::command]
pub async fn run_install_operation(app: AppHandle, kind: String) -> AppResult<InstallOutcome> {
    services::run_install(&app, &kind).await
}

#[tauri::command]
pub async fn verify_installation(app: AppHandle, state: State<'_, AppState>, full: bool) -> AppResult<deploy::VerifyReport> {
    let active = state.install_state().active.ok_or_else(|| AppError::NotFound("No Aeolus-managed installation to verify".into()))?;
    let (cancel, _g) = state.begin_operation("Verifying installation")?;
    changed(&app, "operation");
    let emitter = app.clone();
    let dir = std::path::PathBuf::from(&active.path);
    let report = tokio::task::spawn_blocking(move || {
        deploy::verify(&dir, full, &cancel, &|i, n| {
            use tauri::Emitter;
            let _ = emitter.emit(services::EV_PROGRESS, installation::progress::Progress {
                operation: "Verifying installation".into(),
                stage: if full { "Checking file hashes".into() } else { "Checking files".into() },
                step: i as u32,
                steps: n as u32,
                ..Default::default()
            });
        })
    })
    .await
    .map_err(|e| AppError::Other(e.to_string()))??;
    let summary = if report.ok() {
        format!("All {} files are intact", report.checked)
    } else {
        format!("{} missing, {} damaged", report.missing.len(), report.modified.len())
    };
    state.logger.info("install", format!("Verification ({}) finished: {summary}", if full { "full" } else { "quick" }));
    state.activity.record(if report.ok() { ActivityKind::Install } else { ActivityKind::Error }, "Installation verified", Some(summary));
    use tauri::Emitter;
    let _ = app.emit(services::EV_PROGRESS, installation::progress::Progress { operation: "Verifying installation".into(), stage: "Completed".into(), done: true, ..Default::default() });
    changed(&app, "install");
    Ok(report)
}

#[tauri::command]
pub fn uninstall_roblox(app: AppHandle, state: State<'_, AppState>, clear_cache: bool) -> AppResult<()> {
    let active = state.install_state().active.ok_or_else(|| AppError::NotFound("No Aeolus-managed installation to uninstall".into()))?;
    let (_c, _g) = state.begin_operation("Uninstalling Roblox")?;
    if state.settings().installation.backup_before_destructive {
        let n = state.backup_user_config("before-uninstall")?;
        state.activity.record(ActivityKind::Backup, "Configuration backed up before uninstall", Some(format!("{n} files")));
    }
    // Keep a copy of the applied Fast Flags file; it is the only user-authored file in the folder.
    let client_settings = std::path::Path::new(&active.path).join("ClientSettings").join("ClientAppSettings.json");
    state.backups.create_from_file("client-app-settings", "before-uninstall", &client_settings, 20)?;
    installation::remove_managed(std::path::Path::new(&active.path))?;
    if clear_cache {
        installation::clear_cache(&state.paths.downloads())?;
    }
    state.update_install(|s| s.active = None)?;
    state.logger.info("install", format!("Uninstalled Roblox {}", active.version));
    state.activity.record(ActivityKind::Install, format!("Uninstalled Roblox {}", active.version), Some("Settings, profiles and Fast Flags were kept".into()));
    changed(&app, "install");
    Ok(())
}

#[tauri::command]
pub fn discard_incomplete(app: AppHandle, state: State<'_, AppState>, version_guid: String) -> AppResult<()> {
    if !installation::manifest::is_valid_guid(&version_guid) {
        return Err(AppError::Validation("invalid version id".into()));
    }
    let dir = state.managed_root().join(format!("{version_guid}{}", deploy::STAGING_SUFFIX));
    installation::remove_managed(&dir)?;
    changed(&app, "install");
    Ok(())
}

#[tauri::command]
pub fn clear_download_cache(state: State<'_, AppState>) -> AppResult<u64> {
    if state.current_operation().is_some() {
        return Err(AppError::Busy(state.current_operation().unwrap_or_default()));
    }
    let n = installation::clear_cache(&state.paths.downloads())?;
    state.logger.info("install", format!("Cleared download cache ({n} bytes)"));
    Ok(n)
}

/// Prefer a detected (non-Aeolus) installation for launching, or `None` for Aeolus's own.
#[tauri::command]
pub fn set_launch_install(app: AppHandle, state: State<'_, AppState>, path: Option<String>) -> AppResult<()> {
    if let Some(p) = &path {
        let det = detection(&state);
        let found = det.installs.iter().find(|i| &i.path == p).ok_or_else(|| AppError::NotFound("That installation was not detected".into()))?;
        if !found.valid {
            return Err(AppError::Validation(format!("That installation has problems: {}", found.issues.join(", "))));
        }
        if found.managed {
            state.update_install(|s| s.preferred_external = None)?;
            changed(&app, "install");
            return Ok(());
        }
    }
    state.update_install(|s| s.preferred_external = path.clone())?;
    state.logger.info("install", format!("Launch installation set to {}", path.as_deref().unwrap_or("Aeolus-managed")));
    changed(&app, "install");
    Ok(())
}

/// Change the folder used for Aeolus-managed installations. The existing installation
/// is not moved; Aeolus will install into the new folder on the next install/update.
#[tauri::command]
pub fn set_install_root(app: AppHandle, state: State<'_, AppState>, path: Option<String>) -> AppResult<()> {
    if let Some(p) = &path {
        let pb = std::path::PathBuf::from(p);
        if !pb.is_absolute() {
            return Err(AppError::Validation("Choose an absolute folder path".into()));
        }
        std::fs::create_dir_all(&pb)?;
        let probe = pb.join(".aeolus-write-test");
        std::fs::write(&probe, b"ok").map_err(|e| AppError::Validation(format!("Aeolus cannot write to that folder: {e}")))?;
        let _ = std::fs::remove_file(probe);
    }
    let mut s = state.settings();
    s.installation.custom_install_root = path;
    state.save_settings(s)?;
    changed(&app, "install");
    Ok(())
}

#[tauri::command]
pub fn update_history(state: State<'_, AppState>) -> Vec<UpdateRecord> {
    state.updates.list()
}

#[tauri::command]
pub async fn launch_roblox(app: AppHandle, profile_id: Option<String>) -> AppResult<LaunchRecord> {
    services::launch(&app, None, profile_id).await
}

#[tauri::command]
pub fn launch_history(state: State<'_, AppState>) -> Vec<LaunchRecord> {
    state.launches.list()
}

#[tauri::command]
pub fn clear_launch_history(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.launches.clear()?;
    changed(&app, "launch");
    Ok(())
}
