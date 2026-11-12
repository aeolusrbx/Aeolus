//! Game history and rejoin commands.

use crate::error::{AppError, AppResult};
use crate::gamewatch::GameVisit;
use crate::launcher::LaunchRecord;
use crate::services::{self, changed};
use crate::state::AppState;
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GamesOverview {
    pub current: Option<GameVisit>,
    pub history: Vec<GameVisit>,
    pub tracking: bool,
}

#[tauri::command]
pub fn games_overview(state: State<'_, AppState>) -> GamesOverview {
    GamesOverview {
        current: state.current_game.lock().map(|g| g.clone()).unwrap_or(None),
        history: state.game_history.list(),
        tracking: state.settings().games.track_games,
    }
}

#[tauri::command]
pub fn game_history_clear(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.game_history.clear()?;
    state.logger.info("games", "Game history cleared");
    changed(&app, "games");
    Ok(())
}

/// Join a game again. `same_server` rejoins the exact server (public servers only).
#[tauri::command]
pub async fn game_rejoin(app: AppHandle, state: State<'_, AppState>, id: String, same_server: bool) -> AppResult<LaunchRecord> {
    let v = state
        .game_history
        .list()
        .into_iter()
        .find(|v| v.id == id)
        .ok_or_else(|| AppError::NotFound("That game is no longer in your history".into()))?;
    if same_server && !v.can_rejoin() {
        return Err(AppError::Validation("Private and reserved servers can't be rejoined directly. Use Play to join the game instead.".into()));
    }
    let link = if same_server { v.rejoin_link() } else { v.play_link() };
    state.logger.info("games", format!("{} place {}", if same_server { "Rejoining server in" } else { "Playing" }, v.place_id));
    services::launch(&app, Some(link), None).await
}
