//! Window, settings, startup and self-update commands.

use crate::configuration::settings::Settings;
use crate::diagnostics::activity::{ActivityEntry, ActivityKind};
use crate::error::{AppError, AppResult};
use crate::services::{self, changed};
use crate::state::{AppState, PROTOCOL_BACKUP_FILE};
use crate::system::{protocol, selfupdate};
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub config_dir: String,
    pub data_dir: String,
    pub logs_dir: String,
    pub backups_dir: String,
    pub install_root: String,
    pub startup_notes: Vec<String>,
    pub platform: &'static str,
    pub operation: Option<String>,
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> AppInfo {
    AppInfo {
        name: "Aeolus",
        version: env!("CARGO_PKG_VERSION"),
        config_dir: state.paths.config.to_string_lossy().into(),
        data_dir: state.paths.local.to_string_lossy().into(),
        logs_dir: state.paths.logs().to_string_lossy().into(),
        backups_dir: state.paths.backups().to_string_lossy().into(),
        install_root: state.managed_root().to_string_lossy().into(),
        startup_notes: state.startup_notes.lock().map(|n| n.clone()).unwrap_or_default(),
        platform: std::env::consts::OS,
        operation: state.current_operation(),
    }
}

/// Called by the frontend once it has rendered; shows the window unless Aeolus
/// was started minimized.
#[tauri::command]
pub fn app_ready(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    if let Some(w) = app.get_webview_window("main") {
        if !state.started_minimized.load(Ordering::SeqCst) {
            w.show().map_err(|e| AppError::Other(e.to_string()))?;
            let _ = w.set_focus();
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> AppResult<Settings> {
    let old = state.settings();
    let mut new = settings;
    new.schema_version = old.schema_version;
    // Side effects that must succeed before the setting is persisted.
    if new.general.launch_at_startup != old.general.launch_at_startup {
        let al = app.autolaunch();
        let r = if new.general.launch_at_startup { al.enable() } else { al.disable() };
        r.map_err(|e| AppError::Other(format!("Could not change startup registration: {e}")))?;
        state.logger.info("system", format!("Launch at startup {}", if new.general.launch_at_startup { "enabled" } else { "disabled" }));
    }
    if new.advanced.handle_roblox_links != old.advanced.handle_roblox_links {
        let exe = std::env::current_exe()?;
        let backup = state.paths.file(PROTOCOL_BACKUP_FILE);
        if new.advanced.handle_roblox_links {
            protocol::register(&exe, &backup)?;
        } else {
            protocol::unregister(&exe, &backup)?;
        }
        state.logger.info("system", format!("Roblox link handling {}", if new.advanced.handle_roblox_links { "registered" } else { "unregistered" }));
    }
    state.save_settings(new.clone())?;
    if old.privacy.log_retention_days != new.privacy.log_retention_days {
        state.logger.enforce_retention(new.privacy.log_retention_days);
    }
    state.logger.info("config", "Settings saved");
    state.activity.record(ActivityKind::Config, "Settings changed", None);
    changed(&app, "settings");
    Ok(new)
}

#[tauri::command]
pub fn reset_settings(app: AppHandle, state: State<'_, AppState>) -> AppResult<Settings> {
    let old = state.settings();
    let mut d = Settings::default();
    // keep system registrations consistent with what is actually registered
    d.general.launch_at_startup = old.general.launch_at_startup;
    d.advanced.handle_roblox_links = old.advanced.handle_roblox_links;
    state.save_settings(d.clone())?;
    state.activity.record(ActivityKind::Config, "Settings reset to defaults", Some("Previous settings were backed up".into()));
    changed(&app, "settings");
    Ok(d)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemIntegration {
    pub launch_at_startup: bool,
    pub protocol: protocol::ProtocolStatus,
}

#[tauri::command]
pub fn system_integration(app: AppHandle) -> AppResult<SystemIntegration> {
    let exe = std::env::current_exe()?;
    Ok(SystemIntegration {
        launch_at_startup: app.autolaunch().is_enabled().unwrap_or(false),
        protocol: protocol::status(&exe),
    })
}

#[tauri::command]
pub fn activity_list(state: State<'_, AppState>) -> Vec<ActivityEntry> {
    state.activity.list()
}

#[tauri::command]
pub fn activity_clear(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.activity.clear()?;
    state.logger.info("app", "Activity history cleared");
    changed(&app, "activity");
    Ok(())
}

#[tauri::command]
pub async fn self_update_check(state: State<'_, AppState>) -> AppResult<selfupdate::SelfUpdateCheck> {
    let feed = state.settings().advanced.self_update_feed;
    let r = selfupdate::check(&state.http, &feed, env!("CARGO_PKG_VERSION")).await;
    if let Err(e) = &r {
        state.logger.warn("self-update", format!("Check failed: {e}"));
    }
    r
}

/// Download and verify the installer, then start it and exit Aeolus so no file
/// is replaced while in use. Configuration lives in AppData and is preserved.
#[tauri::command]
pub async fn self_update_install(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    let feed = state.settings().advanced.self_update_feed;
    let check = selfupdate::check(&state.http, &feed, env!("CARGO_PKG_VERSION")).await?;
    let rel = check.latest.filter(|_| check.update_available).ok_or_else(|| AppError::Validation("No newer Aeolus version is available".into()))?;
    let (_c, _g) = state.begin_operation("Updating Aeolus")?;
    let emitter = app.clone();
    let path = selfupdate::download(&state.http, &rel, &state.paths.updates_cache(), move |got, total| {
        use tauri::Emitter;
        let _ = emitter.emit(services::EV_PROGRESS, crate::installation::progress::Progress {
            operation: "Updating Aeolus".into(),
            stage: "Downloading".into(),
            downloaded: got,
            total,
            ..Default::default()
        });
    })
    .await?;
    state.logger.info("self-update", format!("Verified installer for {}; starting it", rel.version));
    state.backup_user_config("before-self-update")?;
    let is_msi = path.extension().map(|e| e == "msi").unwrap_or(false);
    let mut cmd = if is_msi {
        let mut c = std::process::Command::new("msiexec");
        c.arg("/i").arg(&path);
        c
    } else {
        std::process::Command::new(&path)
    };
    cmd.spawn().map_err(|e| AppError::Other(format!("Could not start the installer: {e}")))?;
    app.exit(0);
    Ok(())
}

#[tauri::command]
pub fn cancel_operation(state: State<'_, AppState>) -> bool {
    let r = state.cancel_operation();
    if r {
        state.logger.info("app", "Cancellation requested");
    }
    r
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

/// Close button: hide to tray when enabled, otherwise exit.
#[tauri::command]
pub fn close_window(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    if state.settings().general.minimize_to_tray {
        if let Some(w) = app.get_webview_window("main") {
            w.hide().map_err(|e| AppError::Other(e.to_string()))?;
        }
    } else {
        app.exit(0);
    }
    Ok(())
}

/// Export all Aeolus configuration as one JSON document.
#[tauri::command]
pub fn export_all_data(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let doc = serde_json::json!({
        "aeolusExport": 1,
        "exportedAt": chrono::Local::now(),
        "settings": state.settings(),
        "profiles": state.profiles_snapshot(),
        "fastFlags": state.flags_snapshot(),
        "launchHistory": state.launches.list(),
        "updateHistory": state.updates.list(),
        "activity": state.activity.list(),
    });
    crate::configuration::store::write_json(std::path::Path::new(&path), &doc)?;
    state.logger.info("privacy", "Exported all data");
    Ok(())
}

/// Import a full export (settings, profiles, Fast Flags). Current data is backed up first.
#[tauri::command]
pub fn import_all_data(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let bytes = std::fs::read(&path)?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| AppError::Validation(format!("Not a valid export: {e}")))?;
    if v.get("aeolusExport").and_then(|x| x.as_u64()) != Some(1) {
        return Err(AppError::Validation("This file is not an Aeolus export".into()));
    }
    let settings: Settings = serde_json::from_value(v["settings"].clone()).map_err(|e| AppError::Validation(format!("settings: {e}")))?;
    settings.validate().map_err(AppError::Validation)?;
    let profiles: crate::profiles::ProfileStore = serde_json::from_value(v["profiles"].clone()).map_err(|e| AppError::Validation(format!("profiles: {e}")))?;
    let flags: crate::fastflags::FlagStore = serde_json::from_value(v["fastFlags"].clone()).map_err(|e| AppError::Validation(format!("fastFlags: {e}")))?;
    state.backup_user_config("before-import")?;
    let mut s = settings;
    let cur = state.settings();
    s.general.launch_at_startup = cur.general.launch_at_startup;
    s.advanced.handle_roblox_links = cur.advanced.handle_roblox_links;
    state.save_settings(s)?;
    state.update_profiles(|p| {
        *p = profiles;
        Ok(())
    })?;
    state.update_flags("before-import", |f| {
        *f = flags;
        Ok(())
    })?;
    state.activity.record(ActivityKind::Config, "Imported configuration", Some("Previous configuration was backed up".into()));
    changed(&app, "all");
    Ok(())
}

/// Delete history, logs and activity (configuration is kept unless `include_config`).
#[tauri::command]
pub fn delete_user_data(app: AppHandle, state: State<'_, AppState>, include_config: bool) -> AppResult<()> {
    state.activity.clear()?;
    state.launches.clear()?;
    state.updates.clear()?;
    state.logger.clear()?;
    if include_config {
        state.backup_user_config("before-delete")?;
        let cur = state.settings();
        let mut d = Settings::default();
        d.general.launch_at_startup = cur.general.launch_at_startup;
        d.advanced.handle_roblox_links = cur.advanced.handle_roblox_links;
        state.save_settings(d)?;
        state.update_profiles(|p| {
            *p = Default::default();
            Ok(())
        })?;
        state.update_flags("before-delete", |f| {
            *f = Default::default();
            Ok(())
        })?;
    }
    state.logger.info("privacy", format!("User data deleted (configuration {})", if include_config { "reset" } else { "kept" }));
    changed(&app, "all");
    Ok(())
}

#[tauri::command]
pub fn list_backups(state: State<'_, AppState>, category: String) -> AppResult<Vec<crate::configuration::store::BackupEntry>> {
    state.backups.list(&category)
}
