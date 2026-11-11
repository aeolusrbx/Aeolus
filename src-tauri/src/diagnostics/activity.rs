//! Persistent recent-activity feed shown on the Home page.

use crate::configuration::store;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_ENTRIES: usize = 300;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ActivityKind {
    Launch,
    Update,
    Install,
    Config,
    Backup,
    Plugin,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityEntry {
    pub id: String,
    pub kind: ActivityKind,
    pub title: String,
    pub detail: Option<String>,
    pub at: chrono::DateTime<chrono::Local>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ActivityFile {
    schema_version: u32,
    entries: Vec<ActivityEntry>,
}

pub struct Activity {
    path: PathBuf,
    entries: Mutex<Vec<ActivityEntry>>,
}

impl Activity {
    pub fn load(path: PathBuf) -> Self {
        let entries = store::load_json_or_default::<ActivityFile>(&path).map(|l| l.value.entries).unwrap_or_default();
        Self { path, entries: Mutex::new(entries) }
    }

    pub fn record(&self, kind: ActivityKind, title: impl Into<String>, detail: Option<String>) {
        let mut g = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        g.insert(
            0,
            ActivityEntry {
                id: uuid::Uuid::new_v4().to_string(),
                kind,
                title: title.into(),
                detail: detail.map(|d| crate::diagnostics::logging::redact(&d)),
                at: chrono::Local::now(),
            },
        );
        g.truncate(MAX_ENTRIES);
        let _ = store::write_json(&self.path, &ActivityFile { schema_version: 1, entries: g.clone() });
    }

    pub fn list(&self) -> Vec<ActivityEntry> {
        self.entries.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn clear(&self) -> crate::error::AppResult<()> {
        let mut g = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        g.clear();
        store::write_json(&self.path, &ActivityFile { schema_version: 1, entries: vec![] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn persists_and_clears() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("activity.json");
        let a = Activity::load(p.clone());
        a.record(ActivityKind::Launch, "Launched", None);
        a.record(ActivityKind::Error, "Failed", Some("token=abc".into()));
        let b = Activity::load(p.clone());
        assert_eq!(b.list().len(), 2);
        assert_eq!(b.list()[0].title, "Failed");
        assert!(!b.list()[0].detail.as_ref().unwrap().contains("abc"));
        b.clear().unwrap();
        assert!(Activity::load(p).list().is_empty());
    }
}
