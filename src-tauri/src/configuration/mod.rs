pub mod history;
pub mod migrations;
pub mod settings;
pub mod store;

use crate::error::{AppError, AppResult};
use settings::{Settings, SETTINGS_SCHEMA_VERSION};
use std::path::Path;

/// Load settings with migration, validation and corruption recovery.
/// Returns the settings and a list of human-readable notes about what happened.
pub fn load_settings(path: &Path, backups: &store::Backups) -> AppResult<(Settings, Vec<String>)> {
    let mut notes = vec![];
    let raw = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let s = Settings::default();
            store::write_json(path, &s)?;
            return Ok((s, notes));
        }
        Err(e) => return Err(e.into()),
    };
    let value: serde_json::Value = match serde_json::from_slice(&raw) {
        Ok(v) => v,
        Err(e) => {
            backups.create("settings-corrupt", "unreadable", &raw, 20)?;
            notes.push(format!("settings.json was unreadable ({e}); a copy was saved to backups and defaults were restored"));
            let s = Settings::default();
            store::write_json(path, &s)?;
            return Ok((s, notes));
        }
    };
    let (value, outcome) = migrations::migrate(value, SETTINGS_SCHEMA_VERSION, migrations::SETTINGS_MIGRATIONS)
        .map_err(|e| AppError::Validation(e.to_string()))?;
    if let migrations::MigrationOutcome::Migrated { from, to } = outcome {
        backups.create("settings", &format!("pre-migration-v{from}"), &raw, 20)?;
        notes.push(format!("settings migrated from schema {from} to {to}"));
    }
    let mut s: Settings = match serde_json::from_value(value) {
        Ok(s) => s,
        Err(e) => {
            backups.create("settings-corrupt", "invalid-shape", &raw, 20)?;
            notes.push(format!("settings.json had invalid values ({e}); defaults were restored and the old file backed up"));
            Settings::default()
        }
    };
    if let Err(e) = s.validate() {
        backups.create("settings-corrupt", "failed-validation", &raw, 20)?;
        notes.push(format!("settings failed validation ({e}); defaults were restored and the old file backed up"));
        s = Settings::default();
    }
    s.schema_version = SETTINGS_SCHEMA_VERSION;
    store::write_json(path, &s)?;
    Ok((s, notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_creates_defaults_and_recovers() {
        let d = tempfile::tempdir().unwrap();
        let b = store::Backups::new(d.path().join("backups"));
        let p = d.path().join("settings.json");
        let (s, notes) = load_settings(&p, &b).unwrap();
        assert_eq!(s, Settings::default());
        assert!(notes.is_empty());
        assert!(p.exists());

        // partial file: missing sections are filled with defaults
        std::fs::write(&p, br##"{"schemaVersion":2,"general":{"accent":"#00AAFF"}}"##).unwrap();
        let (s, _) = load_settings(&p, &b).unwrap();
        assert_eq!(s.general.accent, "#00AAFF");
        assert_eq!(s.general.default_page, "home");

        // invalid value -> defaults, backup made
        std::fs::write(&p, br#"{"schemaVersion":2,"general":{"accent":"orange"}}"#).unwrap();
        let (s, notes) = load_settings(&p, &b).unwrap();
        assert_eq!(s.general.accent, "#FF861F");
        assert_eq!(notes.len(), 1);
        assert_eq!(b.list("settings-corrupt").unwrap().len(), 1);

        // garbage -> defaults, backup made
        std::fs::write(&p, b"\x00\x01garbage").unwrap();
        let (_, notes) = load_settings(&p, &b).unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(b.list("settings-corrupt").unwrap().len(), 2);
    }

    #[test]
    fn newer_schema_is_rejected_not_overwritten() {
        let d = tempfile::tempdir().unwrap();
        let b = store::Backups::new(d.path().join("backups"));
        let p = d.path().join("settings.json");
        std::fs::write(&p, br#"{"schemaVersion":99}"#).unwrap();
        assert!(load_settings(&p, &b).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), br#"{"schemaVersion":99}"#);
    }
}
