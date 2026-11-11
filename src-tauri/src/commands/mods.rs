//! Mods (sounds, cursors, fonts) commands.

use crate::diagnostics::activity::ActivityKind;
use crate::error::AppResult;
use crate::mods::{self, ModChoice, ModStore, Slot};
use crate::services::{changed, reapply_content};
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotView {
    #[serde(flatten)]
    pub slot: Slot,
    pub choice: Option<ModChoice>,
    pub preview_url: Option<String>,
    pub preset_previews: Vec<(String, Option<String>)>,
}

#[tauri::command]
pub fn mods_overview(state: State<'_, AppState>) -> Vec<SlotView> {
    let f = state.mods.load();
    mods::SLOTS
        .iter()
        .map(|s| SlotView {
            slot: s.clone(),
            choice: f.slots.get(s.id).cloned(),
            preview_url: state.mods.preview(s.id),
            preset_previews: s.presets.iter().map(|p| (p.id.to_string(), ModStore::preset_preview(p.id))).collect(),
        })
        .collect()
}

fn after(app: &AppHandle, state: &AppState, what: String) -> AppResult<()> {
    reapply_content(state)?;
    state.logger.info("mods", &what);
    state.activity.record(ActivityKind::Config, what, None);
    changed(app, "mods");
    Ok(())
}

#[tauri::command]
pub fn mod_set_file(app: AppHandle, state: State<'_, AppState>, slot: String, path: String) -> AppResult<()> {
    let c = state.mods.set_file(&slot, std::path::Path::new(&path))?;
    let label = mods::slot(&slot)?.label;
    after(&app, &state, format!("{label} set to {}", c.original_name.unwrap_or_default()))
}

#[tauri::command]
pub fn mod_set_preset(app: AppHandle, state: State<'_, AppState>, slot: String, preset: String) -> AppResult<()> {
    state.mods.set_preset(&slot, &preset)?;
    let s = mods::slot(&slot)?;
    let name = s.presets.iter().find(|p| p.id == preset).map(|p| p.label).unwrap_or("preset");
    after(&app, &state, format!("{} set to {name}", s.label))
}

#[tauri::command]
pub fn mod_set_enabled(app: AppHandle, state: State<'_, AppState>, slot: String, enabled: bool) -> AppResult<()> {
    state.mods.set_enabled(&slot, enabled)?;
    let label = mods::slot(&slot)?.label;
    after(&app, &state, format!("{label} {}", if enabled { "turned on" } else { "turned off" }))
}

#[tauri::command]
pub fn mod_clear(app: AppHandle, state: State<'_, AppState>, slot: String) -> AppResult<()> {
    state.mods.clear(&slot)?;
    let label = mods::slot(&slot)?.label;
    after(&app, &state, format!("{label} reset to Roblox's original"))
}

#[tauri::command]
pub fn mods_reset_all(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.mods.clear_all()?;
    after(&app, &state, "All mods reset to Roblox's originals".into())
}
