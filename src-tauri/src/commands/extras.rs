//! Diagnostics and plugin commands.

use crate::diagnostics::activity::ActivityKind;
use crate::diagnostics::logging::{redact, LogEntry, LogQuery, Level};
use crate::diagnostics::{system_info, SystemInfo};
use crate::error::{AppError, AppResult};
use crate::plugins::PluginInfo;
use crate::services::{self, changed};
use crate::state::AppState;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn diagnostics_system() -> AppResult<SystemInfo> {
    tokio::task::spawn_blocking(system_info).await.map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
pub fn logs_query(state: State<'_, AppState>, query: LogQuery) -> AppResult<Vec<LogEntry>> {
    state.logger.query(&query)
}

#[tauri::command]
pub fn logs_clear(state: State<'_, AppState>) -> AppResult<usize> {
    let n = state.logger.clear()?;
    state.logger.info("diagnostics", "Logs cleared");
    Ok(n)
}

#[tauri::command]
pub fn logs_export(state: State<'_, AppState>, query: LogQuery, path: String) -> AppResult<usize> {
    let entries = state.logger.query(&LogQuery { limit: Some(query.limit.unwrap_or(20000)), ..query })?;
    let mut text = String::new();
    for e in entries.iter().rev() {
        text.push_str(&format!("{} [{}] {}: {}\n", e.ts.format("%Y-%m-%d %H:%M:%S"), e.level.as_str(), e.source, e.message));
    }
    crate::configuration::store::atomic_write(std::path::Path::new(&path), text.as_bytes())?;
    Ok(entries.len())
}

/// Build a diagnostic report and write it to `path`. Contains no credentials,
/// cookies or tokens; paths have the Windows user name removed.
#[tauri::command]
pub async fn diagnostics_report(state: State<'_, AppState>, path: String, display: Option<String>) -> AppResult<()> {
    let sys = tokio::task::spawn_blocking(system_info).await.map_err(|e| AppError::Other(e.to_string()))?;
    let st = state.install_state();
    let s = state.settings();
    let errors = state.logger.query(&LogQuery { levels: Some(vec![Level::Error, Level::Warning]), limit: Some(50), ..Default::default() })?;
    let recent = if s.privacy.include_logs_in_reports {
        state.logger.query(&LogQuery { limit: Some(300), ..Default::default() })?
    } else {
        vec![]
    };
    let report = serde_json::json!({
        "aeolusDiagnosticReport": 1,
        "generatedAt": chrono::Local::now(),
        "aeolusVersion": env!("CARGO_PKG_VERSION"),
        "system": sys,
        "display": display,
        "roblox": {
            "installed": st.active.as_ref().map(|a| &a.version),
            "versionGuid": st.active.as_ref().map(|a| &a.version_guid),
            "fileCount": st.active.as_ref().map(|a| a.file_count),
            "usingExternalInstall": st.preferred_external.is_some(),
            "latest": st.last_check.latest,
            "lastCheckError": st.last_check.error,
        },
        "settings": {
            "theme": s.general.theme,
            "debugLogging": s.advanced.debug_logging,
            "handleRobloxLinks": s.advanced.handle_roblox_links,
            "updateBeforeLaunch": s.installation.update_before_launch,
        },
        "recentLaunches": state.launches.list().into_iter().take(10).collect::<Vec<_>>(),
        "recentUpdates": state.updates.list().into_iter().take(10).collect::<Vec<_>>(),
        "recentErrors": errors,
        "logs": recent,
    });
    let text = redact(&serde_json::to_string_pretty(&report)?);
    crate::configuration::store::atomic_write(std::path::Path::new(&path), text.as_bytes())?;
    state.logger.info("diagnostics", "Diagnostic report exported");
    Ok(())
}

#[tauri::command]
pub fn plugins_list(state: State<'_, AppState>) -> Vec<PluginInfo> {
    let req = state.settings().plugins.require_reapproval_on_change;
    state.plugins().list(req)
}

#[tauri::command]
pub fn plugin_enable(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    let m = state.plugins().approve_and_enable(&id)?;
    state.logger.info("plugins", format!("Enabled plugin {} {} with permissions {:?}", m.name, m.version, m.permissions));
    state.activity.record(ActivityKind::Plugin, format!("Enabled plugin {}", m.name), None);
    services::run_hooks(&app, "onEnable", Some(&id));
    changed(&app, "plugins");
    Ok(())
}

#[tauri::command]
pub fn plugin_disable(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    services::run_hooks(&app, "onDisable", Some(&id));
    let m = state.plugins().disable(&id)?;
    let name = m.map(|m| m.name).unwrap_or(id);
    state.logger.info("plugins", format!("Disabled plugin {name}"));
    state.activity.record(ActivityKind::Plugin, format!("Disabled plugin {name}"), None);
    changed(&app, "plugins");
    Ok(())
}

#[tauri::command]
pub fn plugin_set_config(app: AppHandle, state: State<'_, AppState>, id: String, key: String, value: serde_json::Value) -> AppResult<()> {
    state.plugins().set_config(&id, &key, value)?;
    changed(&app, "plugins");
    Ok(())
}

#[tauri::command]
pub fn plugin_install(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<String> {
    let m = state.plugins().install_from(std::path::Path::new(&path))?;
    state.logger.info("plugins", format!("Installed plugin {} {} (disabled until approved)", m.name, m.version));
    state.activity.record(ActivityKind::Plugin, format!("Installed plugin {}", m.name), Some("Awaiting approval".into()));
    changed(&app, "plugins");
    Ok(m.name)
}

#[tauri::command]
pub fn plugin_uninstall(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    let was_active = state.plugins().active().iter().any(|(m, _)| m.id == id);
    if was_active {
        services::run_hooks(&app, "onDisable", Some(&id));
    }
    state.plugins().uninstall(&id)?;
    state.logger.info("plugins", format!("Uninstalled plugin {id}"));
    state.activity.record(ActivityKind::Plugin, format!("Uninstalled plugin {id}"), None);
    changed(&app, "plugins");
    Ok(())
}
