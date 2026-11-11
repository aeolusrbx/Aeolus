//! Roblox client update tracking.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum UpdateResult {
    Success,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateRecord {
    pub id: String,
    /// "install" | "update" | "repair" | "reinstall"
    pub kind: String,
    pub from_version: Option<String>,
    pub to_version: String,
    pub at: chrono::DateTime<chrono::Local>,
    pub result: UpdateResult,
    pub error: Option<String>,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub installed: Option<String>,
    pub installed_guid: Option<String>,
    pub latest: Option<String>,
    pub latest_guid: Option<String>,
    pub update_available: bool,
    pub checked_at: Option<chrono::DateTime<chrono::Local>>,
    pub error: Option<String>,
}

pub fn status(state: &crate::installation::InstallState) -> UpdateStatus {
    let latest = state.last_check.latest.as_ref();
    let installed = state.active.as_ref();
    UpdateStatus {
        installed: installed.map(|i| i.version.clone()),
        installed_guid: installed.map(|i| i.version_guid.clone()),
        latest: latest.map(|l| l.version.clone()),
        latest_guid: latest.map(|l| l.version_guid.clone()),
        update_available: matches!((installed, latest), (Some(i), Some(l)) if i.version_guid != l.version_guid),
        checked_at: state.last_check.checked_at,
        error: state.last_check.error.clone(),
    }
}
