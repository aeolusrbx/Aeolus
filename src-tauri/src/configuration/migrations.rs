//! Versioned configuration migrations.
//!
//! Each document carries a `schemaVersion`. Migrations are pure functions on
//! `serde_json::Value` that upgrade from version N to N+1. When the schema
//! changes, bump the document's current version and append a migration here.

use serde_json::Value;

pub type Migration = fn(Value) -> Value;

#[derive(Debug, PartialEq)]
pub enum MigrationOutcome {
    UpToDate,
    Migrated { from: u32, to: u32 },
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum MigrationError {
    #[error("this file was written by a newer version of Aeolus (schema {found}, supported {supported})")]
    TooNew { found: u32, supported: u32 },
    #[error("migration for schema {0} is missing")]
    Missing(u32),
}

/// `migrations[i]` upgrades schema `i + 1` to `i + 2` (schema 1 is the first).
pub fn migrate(mut doc: Value, current: u32, migrations: &[Migration]) -> Result<(Value, MigrationOutcome), MigrationError> {
    let found = doc
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(1)
        .max(1);
    if found > current {
        return Err(MigrationError::TooNew { found, supported: current });
    }
    if found == current {
        return Ok((doc, MigrationOutcome::UpToDate));
    }
    let mut v = found;
    while v < current {
        let m = migrations.get((v - 1) as usize).ok_or(MigrationError::Missing(v))?;
        doc = m(doc);
        v += 1;
        if let Some(obj) = doc.as_object_mut() {
            obj.insert("schemaVersion".into(), Value::from(v));
        }
    }
    Ok((doc, MigrationOutcome::Migrated { from: found, to: current }))
}

/// Settings migrations: index 0 upgrades schema 1 -> 2.
pub const SETTINGS_MIGRATIONS: &[Migration] = &[settings_v1_to_v2];

/// v2 (Aeolus 0.1.1): the official release feed became the default. Installs that
/// never set a feed get it; a feed the user typed in is kept.
fn settings_v1_to_v2(mut doc: Value) -> Value {
    if let Some(obj) = doc.as_object_mut() {
        let adv = obj.entry("advanced").or_insert_with(|| Value::Object(Default::default()));
        if let Some(a) = adv.as_object_mut() {
            let empty = a.get("selfUpdateFeed").and_then(|v| v.as_str()).map(|s| s.trim().is_empty()).unwrap_or(true);
            if empty {
                a.insert("selfUpdateFeed".into(), Value::from(super::settings::DEFAULT_UPDATE_FEED));
            }
        }
    }
    doc
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn v1_to_v2(mut d: Value) -> Value {
        // example: move top-level "accent" into general.accent
        let accent = d.as_object_mut().unwrap().remove("accent");
        if let Some(a) = accent {
            d["general"]["accent"] = a;
        }
        d
    }
    fn v2_to_v3(mut d: Value) -> Value {
        d["added"] = json!(true);
        d
    }

    #[test]
    fn migrates_step_by_step() {
        let doc = json!({"schemaVersion": 1, "accent": "#123456", "general": {}});
        let (out, outcome) = migrate(doc, 3, &[v1_to_v2, v2_to_v3]).unwrap();
        assert_eq!(outcome, MigrationOutcome::Migrated { from: 1, to: 3 });
        assert_eq!(out["general"]["accent"], "#123456");
        assert_eq!(out["added"], true);
        assert_eq!(out["schemaVersion"], 3);
    }

    #[test]
    fn rejects_newer_files() {
        let doc = json!({"schemaVersion": 9});
        assert_eq!(migrate(doc, 1, SETTINGS_MIGRATIONS).unwrap_err(), MigrationError::TooNew { found: 9, supported: 1 });
    }

    #[test]
    fn missing_version_is_treated_as_v1() {
        let (_, o) = migrate(json!({}), 1, SETTINGS_MIGRATIONS).unwrap();
        assert_eq!(o, MigrationOutcome::UpToDate);
    }

    #[test]
    fn settings_v2_sets_default_feed_but_keeps_custom() {
        use crate::configuration::settings::DEFAULT_UPDATE_FEED;
        let (d, o) = migrate(json!({"schemaVersion": 1, "advanced": {"selfUpdateFeed": ""}}), 2, SETTINGS_MIGRATIONS).unwrap();
        assert_eq!(o, MigrationOutcome::Migrated { from: 1, to: 2 });
        assert_eq!(d["advanced"]["selfUpdateFeed"], DEFAULT_UPDATE_FEED);
        let (d, _) = migrate(json!({"schemaVersion": 1, "advanced": {"selfUpdateFeed": "https://example.com/f.json"}}), 2, SETTINGS_MIGRATIONS).unwrap();
        assert_eq!(d["advanced"]["selfUpdateFeed"], "https://example.com/f.json");
        let (d, _) = migrate(json!({"schemaVersion": 1}), 2, SETTINGS_MIGRATIONS).unwrap();
        assert_eq!(d["advanced"]["selfUpdateFeed"], DEFAULT_UPDATE_FEED);
    }

    #[test]
    fn missing_migration_errors() {
        assert_eq!(migrate(json!({"schemaVersion": 1}), 2, &[]).unwrap_err(), MigrationError::Missing(1));
    }
}
