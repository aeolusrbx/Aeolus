//! Fast Flags, profiles and backup/restore commands.

use crate::configuration::store::BackupEntry;
use crate::diagnostics::activity::ActivityKind;
use crate::error::{AppError, AppResult};
use crate::fastflags::{Comparison, FlagProfile, FlagRow, FlagStore};
use crate::profiles::{Profile, ProfileInput, ProfileStore};
use crate::services::{changed, resolve_target};
use crate::state::{AppState, FLAGS_FILE, PROFILES_FILE, SETTINGS_FILE};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagProfileSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub count: usize,
    pub modified_at: chrono::DateTime<chrono::Local>,
    pub is_default: bool,
}

#[tauri::command]
pub fn ff_profiles(state: State<'_, AppState>) -> Vec<FlagProfileSummary> {
    let def = state.settings().fast_flags.default_profile_id;
    state
        .flags_snapshot()
        .profiles
        .into_iter()
        .map(|p| FlagProfileSummary { is_default: def.as_deref() == Some(p.id.as_str()), id: p.id, name: p.name, description: p.description, count: p.flags.len(), modified_at: p.modified_at })
        .collect()
}

#[tauri::command]
pub fn ff_rows(state: State<'_, AppState>, profile_id: String) -> AppResult<Vec<FlagRow>> {
    state.flags_snapshot().rows(&profile_id)
}

fn done<T>(app: &AppHandle, state: &AppState, what: &str, r: T) -> T {
    state.logger.info("fastflags", what);
    state.activity.record(ActivityKind::Config, what.to_string(), None);
    changed(app, "fastflags");
    r
}

#[tauri::command]
pub fn ff_create(app: AppHandle, state: State<'_, AppState>, name: String, copy_from: Option<String>) -> AppResult<FlagProfile> {
    let p = state.update_flags("before-create", |s| s.create(&name, "", copy_from.as_deref()))?;
    Ok(done(&app, &state, &format!("Created Fast Flag profile '{}'", p.name), p))
}

#[tauri::command]
pub fn ff_duplicate(app: AppHandle, state: State<'_, AppState>, profile_id: String) -> AppResult<FlagProfile> {
    let p = state.update_flags("before-duplicate", |s| s.duplicate(&profile_id))?;
    Ok(done(&app, &state, &format!("Duplicated Fast Flag profile as '{}'", p.name), p))
}

#[tauri::command]
pub fn ff_rename(app: AppHandle, state: State<'_, AppState>, profile_id: String, name: String, description: Option<String>) -> AppResult<()> {
    state.update_flags("before-rename", |s| s.rename(&profile_id, &name, description.as_deref()))?;
    done(&app, &state, &format!("Renamed Fast Flag profile to '{name}'"), ());
    Ok(())
}

#[tauri::command]
pub fn ff_delete(app: AppHandle, state: State<'_, AppState>, profile_id: String) -> AppResult<()> {
    let removed = state.update_flags("before-delete", |s| s.delete(&profile_id))?;
    let detached = state.update_profiles(|p| Ok(p.detach_flag_profile(&profile_id)))?;
    let mut s = state.settings();
    if s.fast_flags.default_profile_id.as_deref() == Some(profile_id.as_str()) {
        s.fast_flags.default_profile_id = None;
        state.save_settings(s)?;
    }
    if detached > 0 {
        changed(&app, "profiles");
    }
    done(&app, &state, &format!("Deleted Fast Flag profile '{}' (backed up)", removed.name), ());
    Ok(())
}

#[tauri::command]
pub fn ff_set_flag(app: AppHandle, state: State<'_, AppState>, profile_id: String, name: String, value: Value, previous_name: Option<String>) -> AppResult<crate::fastflags::validate::FlagCheck> {
    let block = state.settings().fast_flags.block_invalid_values;
    let c = state.update_flags("before-edit", |s| s.set_flag(&profile_id, &name, &value, previous_name.as_deref(), block))?;
    state.logger.info("fastflags", format!("Set {name}"));
    changed(&app, "fastflags");
    Ok(c)
}

#[tauri::command]
pub fn ff_remove_flags(app: AppHandle, state: State<'_, AppState>, profile_id: String, names: Vec<String>) -> AppResult<usize> {
    let n = state.update_flags("before-remove", |s| s.remove_flags(&profile_id, &names))?;
    Ok(done(&app, &state, &format!("Removed {n} Fast Flag(s)"), n))
}

#[tauri::command]
pub fn ff_reset(app: AppHandle, state: State<'_, AppState>, profile_id: String) -> AppResult<()> {
    state.update_flags("before-reset", |s| s.reset(&profile_id))?;
    done(&app, &state, "Reset a Fast Flag profile (backed up)", ());
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported: usize,
    pub rejected: Vec<String>,
}

#[tauri::command]
pub fn ff_import(app: AppHandle, state: State<'_, AppState>, profile_id: String, json: String, replace: bool) -> AppResult<ImportResult> {
    let block = state.settings().fast_flags.block_invalid_values;
    let (imported, rejected) = state.update_flags("before-import", |s| s.import(&profile_id, &json, replace, block))?;
    Ok(done(&app, &state, &format!("Imported {imported} Fast Flag(s)"), ImportResult { imported, rejected }))
}

#[tauri::command]
pub fn ff_import_file(app: AppHandle, state: State<'_, AppState>, profile_id: String, path: String, replace: bool) -> AppResult<ImportResult> {
    let meta = std::fs::metadata(&path)?;
    if meta.len() > 2 * 1024 * 1024 {
        return Err(AppError::Validation("That file is too large to be a Fast Flag file".into()));
    }
    ff_import(app, state, profile_id, std::fs::read_to_string(&path)?, replace)
}

#[tauri::command]
pub fn ff_export(state: State<'_, AppState>, profile_id: String) -> AppResult<String> {
    state.flags_snapshot().export(&profile_id)
}

#[tauri::command]
pub fn ff_export_file(state: State<'_, AppState>, profile_id: String, path: String) -> AppResult<()> {
    let json = state.flags_snapshot().export(&profile_id)?;
    crate::configuration::store::atomic_write(std::path::Path::new(&path), json.as_bytes())
}

#[tauri::command]
pub fn ff_compare(state: State<'_, AppState>, left: String, right: String) -> AppResult<Comparison> {
    state.flags_snapshot().compare(&left, &right)
}

#[tauri::command]
pub fn ff_set_default(app: AppHandle, state: State<'_, AppState>, profile_id: Option<String>) -> AppResult<()> {
    if let Some(id) = &profile_id {
        state.flags_snapshot().get(id)?;
    }
    let mut s = state.settings();
    s.fast_flags.default_profile_id = profile_id;
    state.save_settings(s)?;
    changed(&app, "fastflags");
    Ok(())
}

/// Write a Fast Flag profile to the current launch installation now.
#[tauri::command]
pub fn ff_apply_now(app: AppHandle, state: State<'_, AppState>, profile_id: String) -> AppResult<usize> {
    let p = state.flags_snapshot().get(&profile_id)?.clone();
    let target = resolve_target(&state)?;
    let n = crate::fastflags::apply_to_install(&target.dir, &p.flags, &state.backups)?;
    Ok(done(&app, &state, &format!("Applied '{}' ({n} flags) to the installation", p.name), n))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPreset {
    pub plugin_id: String,
    pub plugin_name: String,
    pub preset: crate::plugins::FlagPreset,
}

#[tauri::command]
pub fn ff_plugin_presets(state: State<'_, AppState>) -> Vec<PluginPreset> {
    state
        .plugins()
        .active()
        .into_iter()
        .filter(|(_, r)| r.approved_permissions.contains(&crate::plugins::Permission::FastFlagPresets))
        .flat_map(|(m, _)| {
            let (id, name) = (m.id.clone(), m.name.clone());
            m.contributes.fast_flag_presets.into_iter().map(move |p| PluginPreset { plugin_id: id.clone(), plugin_name: name.clone(), preset: p })
        })
        .collect()
}

#[tauri::command]
pub fn ff_allowlist() -> Vec<(String, crate::fastflags::validate::Category)> {
    crate::fastflags::validate::ALLOWLIST.iter().map(|(n, c)| (n.to_string(), *c)).collect()
}

// ---------- profiles ----------

#[tauri::command]
pub fn profiles_get(state: State<'_, AppState>) -> ProfileStore {
    state.profiles_snapshot()
}

fn pdone<T>(app: &AppHandle, state: &AppState, what: String, r: T) -> T {
    state.logger.info("profiles", &what);
    state.activity.record(ActivityKind::Config, what, None);
    changed(app, "profiles");
    r
}

fn check_flag_ref(state: &AppState, input: &ProfileInput) -> AppResult<()> {
    if let Some(id) = &input.fast_flag_profile_id {
        state.flags_snapshot().get(id)?;
    }
    Ok(())
}

#[tauri::command]
pub fn profile_create(app: AppHandle, state: State<'_, AppState>, input: ProfileInput) -> AppResult<Profile> {
    check_flag_ref(&state, &input)?;
    let p = state.update_profiles(|s| s.create(input))?;
    Ok(pdone(&app, &state, format!("Created profile '{}'", p.name), p))
}

#[tauri::command]
pub fn profile_update(app: AppHandle, state: State<'_, AppState>, id: String, input: ProfileInput) -> AppResult<Profile> {
    check_flag_ref(&state, &input)?;
    let p = state.update_profiles(|s| s.update(&id, input))?;
    Ok(pdone(&app, &state, format!("Updated profile '{}'", p.name), p))
}

#[tauri::command]
pub fn profile_duplicate(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<Profile> {
    let p = state.update_profiles(|s| s.duplicate(&id))?;
    Ok(pdone(&app, &state, format!("Duplicated profile as '{}'", p.name), p))
}

#[tauri::command]
pub fn profile_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    let keep = state.settings().installation.keep_backups as usize;
    state.backups.create_from_file("config-profiles", "before-delete", &state.paths.file(PROFILES_FILE), keep)?;
    let p = state.update_profiles(|s| s.delete(&id))?;
    pdone(&app, &state, format!("Deleted profile '{}' (backed up)", p.name), ());
    Ok(())
}

#[tauri::command]
pub fn profile_set_default(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.update_profiles(|s| s.set_default(&id))?;
    let mut st = state.settings();
    st.launch.default_profile_id = Some(id);
    state.save_settings(st)?;
    changed(&app, "profiles");
    Ok(())
}

#[tauri::command]
pub fn profile_set_active(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.update_profiles(|s| s.set_active(&id))?;
    changed(&app, "profiles");
    Ok(())
}

#[tauri::command]
pub fn profile_export_file(state: State<'_, AppState>, id: String, path: String) -> AppResult<()> {
    let json = state.profiles_snapshot().export(&id)?;
    crate::configuration::store::atomic_write(std::path::Path::new(&path), json.as_bytes())
}

#[tauri::command]
pub fn profile_import_file(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<Profile> {
    let text = std::fs::read_to_string(&path)?;
    let p = state.update_profiles(|s| s.import(&text))?;
    Ok(pdone(&app, &state, format!("Imported profile '{}'", p.name), p))
}

// ---------- backups / restore ----------

/// Categories the user can restore from.
const RESTORABLE: &[(&str, &str)] = &[
    ("fastflags", FLAGS_FILE),
    ("config-fastflags", FLAGS_FILE),
    ("settings", SETTINGS_FILE),
    ("config-settings", SETTINGS_FILE),
    ("config-profiles", PROFILES_FILE),
];

#[tauri::command]
pub fn backups_overview(state: State<'_, AppState>) -> AppResult<Vec<BackupEntry>> {
    let mut all = vec![];
    for (c, _) in RESTORABLE {
        all.extend(state.backups.list(c)?);
    }
    all.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(all)
}

#[tauri::command]
pub fn restore_backup(app: AppHandle, state: State<'_, AppState>, category: String, id: String) -> AppResult<()> {
    let (_, file) = RESTORABLE.iter().find(|(c, _)| *c == category).ok_or_else(|| AppError::Validation("That backup cannot be restored".into()))?;
    let bytes = state.backups.read(&category, &id)?;
    match *file {
        FLAGS_FILE => {
            let v: FlagStore = serde_json::from_slice(&bytes).map_err(|e| AppError::Validation(format!("Backup is unreadable: {e}")))?;
            state.update_flags("before-restore", |s| {
                *s = v;
                Ok(())
            })?;
        }
        PROFILES_FILE => {
            let v: ProfileStore = serde_json::from_slice(&bytes).map_err(|e| AppError::Validation(format!("Backup is unreadable: {e}")))?;
            let keep = state.settings().installation.keep_backups as usize;
            state.backups.create_from_file("config-profiles", "before-restore", &state.paths.file(PROFILES_FILE), keep)?;
            state.update_profiles(|s| {
                *s = v;
                Ok(())
            })?;
        }
        SETTINGS_FILE => {
            let mut v: crate::configuration::settings::Settings = serde_json::from_slice(&bytes).map_err(|e| AppError::Validation(format!("Backup is unreadable: {e}")))?;
            let cur = state.settings();
            v.general.launch_at_startup = cur.general.launch_at_startup;
            v.advanced.handle_roblox_links = cur.advanced.handle_roblox_links;
            state.save_settings(v)?;
        }
        _ => unreachable!(),
    }
    state.logger.info("backup", format!("Restored {category} backup {id}"));
    state.activity.record(ActivityKind::Backup, "Configuration restored", Some(format!("{category}: {id}")));
    changed(&app, "all");
    Ok(())
}

#[tauri::command]
pub fn create_manual_backup(state: State<'_, AppState>) -> AppResult<usize> {
    let n = state.backup_user_config("manual")?;
    state.activity.record(ActivityKind::Backup, "Manual configuration backup", Some(format!("{n} files")));
    Ok(n)
}
