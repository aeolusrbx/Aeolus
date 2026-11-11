//! Custom sky commands.

use crate::diagnostics::activity::ActivityKind;
use crate::error::{AppError, AppResult};
use crate::services::{changed, resolve_target};
use crate::skies::{self, SkyPack};
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkyEntry {
    #[serde(flatten)]
    pub pack: SkyPack,
    pub preview_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkiesOverview {
    pub packs: Vec<SkyEntry>,
    /// Sky used by launches without a profile override (None = Roblox's sky).
    pub global_pack_id: Option<String>,
    /// Sky currently written into the launch installation, if any.
    pub applied_pack_id: Option<String>,
    pub applied_pack_name: Option<String>,
}

#[tauri::command]
pub fn skies_overview(state: State<'_, AppState>) -> SkiesOverview {
    let packs = state
        .skies
        .list()
        .into_iter()
        .map(|p| SkyEntry { preview_url: state.skies.preview_data_url(&p.id), pack: p })
        .collect();
    let applied = resolve_target(&state).ok().map(|t| skies::read_overlay(&t.dir)).unwrap_or_default();
    SkiesOverview {
        packs,
        global_pack_id: state.settings().mods.sky_pack_id,
        applied_pack_id: applied.sky_pack_id,
        applied_pack_name: applied.sky_pack_name,
    }
}

#[tauri::command]
pub fn sky_import(app: AppHandle, state: State<'_, AppState>, path: String, name: Option<String>) -> AppResult<SkyPack> {
    let p = state.skies.import(std::path::Path::new(&path), name.as_deref())?;
    state.logger.info("skies", format!("Imported sky '{}' ({} files)", p.name, p.files.len()));
    state.activity.record(ActivityKind::Config, format!("Added sky '{}'", p.name), None);
    changed(&app, "skies");
    Ok(p)
}

#[tauri::command]
pub fn sky_rename(app: AppHandle, state: State<'_, AppState>, id: String, name: String) -> AppResult<()> {
    state.skies.rename(&id, &name)?;
    changed(&app, "skies");
    Ok(())
}

#[tauri::command]
pub fn sky_delete(app: AppHandle, state: State<'_, AppState>, id: String) -> AppResult<()> {
    let p = state.skies.get(&id)?;
    // If it's currently applied, put Roblox's sky back first.
    if let Ok(t) = resolve_target(&state) {
        if skies::read_overlay(&t.dir).sky_pack_id.as_deref() == Some(id.as_str()) {
            skies::restore(&t.dir)?;
        }
    }
    state.skies.delete(&id)?;
    let mut s = state.settings();
    if s.mods.sky_pack_id.as_deref() == Some(id.as_str()) {
        s.mods.sky_pack_id = None;
        state.save_settings(s)?;
    }
    let detached = state.update_profiles(|p| Ok(p.detach_sky(&id)))?;
    if detached > 0 {
        changed(&app, "profiles");
    }
    state.logger.info("skies", format!("Deleted sky '{}'", p.name));
    state.activity.record(ActivityKind::Config, format!("Deleted sky '{}'", p.name), None);
    changed(&app, "skies");
    Ok(())
}

/// Set the global sky (None = Roblox's sky) and apply it to the installation now.
#[tauri::command]
pub fn sky_set_global(app: AppHandle, state: State<'_, AppState>, id: Option<String>) -> AppResult<Option<String>> {
    let pack = match &id {
        Some(i) => Some(state.skies.get(i)?),
        None => None,
    };
    let mut s = state.settings();
    s.mods.sky_pack_id = id.clone();
    state.save_settings(s)?;
    let name = pack.as_ref().map(|p| p.name.clone());
    // Apply right away when there's an installation; otherwise it applies on next launch.
    let applied = match resolve_target(&state) {
        Ok(t) => {
            let active = state.profiles_snapshot().current().and_then(|p| p.sky.clone());
            if active.is_some() {
                None // the active profile has its own sky; leave the install alone
            } else {
                let (n, _) = skies::apply(&t.dir, &state.skies, pack.as_ref())?;
                Some(n)
            }
        }
        Err(_) => None,
    };
    state.logger.info("skies", format!("Global sky set to {} (applied: {})", name.as_deref().unwrap_or("Roblox default"), applied.map(|n| n.to_string()).unwrap_or_else(|| "on next launch".into())));
    changed(&app, "skies");
    changed(&app, "settings");
    if pack.is_none() && id.is_some() {
        return Err(AppError::NotFound("Sky not found".into()));
    }
    Ok(name)
}
