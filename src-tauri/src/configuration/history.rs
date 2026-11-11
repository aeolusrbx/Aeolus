//! Capped, persisted list of records (launch history, update history).

use super::store;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryFile<T> {
    #[serde(default = "one")]
    schema_version: u32,
    #[serde(default = "Vec::new")]
    entries: Vec<T>,
}
fn one() -> u32 {
    1
}
impl<T> Default for HistoryFile<T> {
    fn default() -> Self {
        Self { schema_version: 1, entries: vec![] }
    }
}

pub struct History<T> {
    path: PathBuf,
    cap: usize,
    entries: Mutex<Vec<T>>,
}

impl<T: Serialize + DeserializeOwned + Clone> History<T> {
    pub fn load(path: PathBuf, cap: usize) -> Self {
        let entries = store::load_json_or_default::<HistoryFile<T>>(&path).map(|l| l.value.entries).unwrap_or_default();
        Self { path, cap, entries: Mutex::new(entries) }
    }
    pub fn push(&self, item: T) {
        let mut g = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        g.insert(0, item);
        g.truncate(self.cap);
        let _ = store::write_json(&self.path, &HistoryFile { schema_version: 1, entries: g.clone() });
    }
    pub fn list(&self) -> Vec<T> {
        self.entries.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    pub fn clear(&self) -> crate::error::AppResult<()> {
        let mut g = self.entries.lock().unwrap_or_else(|p| p.into_inner());
        g.clear();
        store::write_json(&self.path, &HistoryFile::<T> { schema_version: 1, entries: vec![] })
    }
}
