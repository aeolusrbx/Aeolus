//! Fast Flag profiles: storage, editing, comparison, import/export and applying
//! a profile to an installation's ClientSettings/ClientAppSettings.json.

pub mod validate;

use crate::configuration::store;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlagProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub flags: BTreeMap<String, Value>,
    pub created_at: chrono::DateTime<chrono::Local>,
    pub modified_at: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct FlagStore {
    pub schema_version: u32,
    pub profiles: Vec<FlagProfile>,
}

fn profile(name: &str, description: &str, flags: &[(&str, Value)]) -> FlagProfile {
    let now = chrono::Local::now();
    FlagProfile {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.into(),
        description: description.into(),
        flags: flags.iter().map(|(k, v)| (k.to_string(), v.clone())).collect(),
        created_at: now,
        modified_at: now,
    }
}

impl Default for FlagStore {
    fn default() -> Self {
        use serde_json::json;
        Self {
            schema_version: 1,
            profiles: vec![
                profile("Default", "No overrides - Roblox's own settings.", &[]),
                profile(
                    "Graphics",
                    "Higher texture quality and anti-aliasing (allowlisted flags only).",
                    &[
                        ("DFFlagTextureQualityOverrideEnabled", json!(true)),
                        ("DFIntTextureQualityOverride", json!(3)),
                        ("FIntDebugForceMSAASamples", json!(4)),
                    ],
                ),
                profile(
                    "Performance",
                    "Lower texture quality, no MSAA and no grass (allowlisted flags only).",
                    &[
                        ("DFFlagTextureQualityOverrideEnabled", json!(true)),
                        ("DFIntTextureQualityOverride", json!(0)),
                        ("FIntDebugForceMSAASamples", json!(0)),
                        ("FIntFRMMinGrassDistance", json!(0)),
                        ("FIntFRMMaxGrassDistance", json!(0)),
                    ],
                ),
                profile("Custom", "Your own configuration.", &[]),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagRow {
    pub name: String,
    pub value: Value,
    #[serde(flatten)]
    pub check: validate::FlagCheck,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Comparison {
    pub only_left: Vec<String>,
    pub only_right: Vec<String>,
    pub changed: Vec<(String, Value, Value)>,
    pub same: usize,
}

impl FlagStore {
    pub fn get(&self, id: &str) -> AppResult<&FlagProfile> {
        self.profiles.iter().find(|p| p.id == id).ok_or_else(|| AppError::NotFound("Fast Flag profile not found".into()))
    }
    fn get_mut(&mut self, id: &str) -> AppResult<&mut FlagProfile> {
        self.profiles.iter_mut().find(|p| p.id == id).ok_or_else(|| AppError::NotFound("Fast Flag profile not found".into()))
    }

    fn check_name(&self, name: &str, except: Option<&str>) -> AppResult<String> {
        let n = name.trim();
        if n.is_empty() || n.len() > 60 {
            return Err(AppError::Validation("Profile names must be 1-60 characters".into()));
        }
        if self.profiles.iter().any(|p| Some(p.id.as_str()) != except && p.name.eq_ignore_ascii_case(n)) {
            return Err(AppError::Validation(format!("A profile named '{n}' already exists")));
        }
        Ok(n.to_string())
    }

    pub fn rows(&self, id: &str) -> AppResult<Vec<FlagRow>> {
        let p = self.get(id)?;
        let names: Vec<&str> = p.flags.keys().map(String::as_str).collect();
        Ok(p.flags
            .iter()
            .map(|(k, v)| FlagRow { name: k.clone(), value: v.clone(), check: validate::check(k, v, &names) })
            .collect())
    }

    pub fn create(&mut self, name: &str, description: &str, copy_from: Option<&str>) -> AppResult<FlagProfile> {
        let name = self.check_name(name, None)?;
        let flags = match copy_from {
            Some(id) => self.get(id)?.flags.clone(),
            None => BTreeMap::new(),
        };
        let mut p = profile(&name, description, &[]);
        p.flags = flags;
        self.profiles.push(p.clone());
        Ok(p)
    }

    pub fn duplicate(&mut self, id: &str) -> AppResult<FlagProfile> {
        let src = self.get(id)?.clone();
        let mut n = 2;
        let name = loop {
            let candidate = format!("{} ({n})", src.name);
            if !self.profiles.iter().any(|p| p.name.eq_ignore_ascii_case(&candidate)) {
                break candidate;
            }
            n += 1;
        };
        self.create(&name, &src.description, Some(id))
    }

    pub fn rename(&mut self, id: &str, name: &str, description: Option<&str>) -> AppResult<()> {
        let name = self.check_name(name, Some(id))?;
        let p = self.get_mut(id)?;
        p.name = name;
        if let Some(d) = description {
            p.description = d.chars().take(300).collect();
        }
        p.modified_at = chrono::Local::now();
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> AppResult<FlagProfile> {
        if self.profiles.len() <= 1 {
            return Err(AppError::Validation("At least one Fast Flag profile must remain".into()));
        }
        let idx = self.profiles.iter().position(|p| p.id == id).ok_or_else(|| AppError::NotFound("Fast Flag profile not found".into()))?;
        Ok(self.profiles.remove(idx))
    }

    /// Add or edit a flag. `previous_name` supports renaming a flag in place.
    pub fn set_flag(&mut self, id: &str, name: &str, value: &Value, previous_name: Option<&str>, block_invalid: bool) -> AppResult<validate::FlagCheck> {
        let name = name.trim();
        validate::valid_name(name).map_err(AppError::Validation)?;
        let coerced = match validate::coerce(name, value) {
            Ok(v) => v,
            Err(e) if block_invalid => return Err(AppError::Validation(e)),
            Err(_) => value.clone(),
        };
        let p = self.get_mut(id)?;
        let renaming = previous_name.filter(|n| *n != name);
        if p.flags.keys().any(|k| k.eq_ignore_ascii_case(name) && Some(k.as_str()) != previous_name) {
            return Err(AppError::Validation(format!("{name} is already in this profile")));
        }
        if let Some(old) = renaming {
            p.flags.remove(old);
        }
        p.flags.insert(name.to_string(), coerced.clone());
        p.modified_at = chrono::Local::now();
        let names: Vec<&str> = p.flags.keys().map(String::as_str).collect();
        let c = validate::check(name, &coerced, &names);
        if block_invalid && c.status == validate::FlagStatus::Invalid {
            return Err(AppError::Validation(c.messages.join("; ")));
        }
        Ok(c)
    }

    pub fn remove_flags(&mut self, id: &str, names: &[String]) -> AppResult<usize> {
        let p = self.get_mut(id)?;
        let before = p.flags.len();
        for n in names {
            p.flags.remove(n);
        }
        p.modified_at = chrono::Local::now();
        Ok(before - p.flags.len())
    }

    pub fn reset(&mut self, id: &str) -> AppResult<()> {
        let p = self.get_mut(id)?;
        p.flags.clear();
        p.modified_at = chrono::Local::now();
        Ok(())
    }

    /// Import a JSON object of flags. Returns (imported, rejected-with-reason).
    pub fn import(&mut self, id: &str, json: &str, replace: bool, block_invalid: bool) -> AppResult<(usize, Vec<String>)> {
        let parsed: Value = serde_json::from_str(json).map_err(|e| AppError::Validation(format!("Not valid JSON: {e}")))?;
        let obj = parsed.as_object().ok_or_else(|| AppError::Validation("Expected a JSON object of flag names to values".into()))?;
        let mut accepted = BTreeMap::new();
        let mut rejected = vec![];
        for (k, v) in obj {
            if let Err(e) = validate::valid_name(k) {
                rejected.push(format!("{k}: {e}"));
                continue;
            }
            match validate::coerce(k, v) {
                Ok(c) => {
                    accepted.insert(k.clone(), c);
                }
                Err(e) if block_invalid => rejected.push(e),
                Err(_) => {
                    accepted.insert(k.clone(), v.clone());
                }
            }
        }
        let p = self.get_mut(id)?;
        if replace {
            p.flags.clear();
        }
        let n = accepted.len();
        for (k, v) in accepted {
            let existing: Vec<String> = p.flags.keys().filter(|e| e.eq_ignore_ascii_case(&k) && **e != k).cloned().collect();
            for e in existing {
                p.flags.remove(&e);
            }
            p.flags.insert(k, v);
        }
        p.modified_at = chrono::Local::now();
        Ok((n, rejected))
    }

    pub fn export(&self, id: &str) -> AppResult<String> {
        Ok(serde_json::to_string_pretty(&self.get(id)?.flags)?)
    }

    pub fn compare(&self, left: &str, right: &str) -> AppResult<Comparison> {
        let (a, b) = (&self.get(left)?.flags, &self.get(right)?.flags);
        let mut c = Comparison::default();
        for (k, v) in a {
            match b.get(k) {
                None => c.only_left.push(k.clone()),
                Some(w) if w != v => c.changed.push((k.clone(), v.clone(), w.clone())),
                Some(_) => c.same += 1,
            }
        }
        c.only_right = b.keys().filter(|k| !a.contains_key(*k)).cloned().collect();
        Ok(c)
    }
}

/// Write the profile's flags to `<install>/ClientSettings/ClientAppSettings.json`,
/// backing up the previous file. Returns the number of flags written.
pub fn apply_to_install(install_dir: &Path, flags: &BTreeMap<String, Value>, backups: &store::Backups) -> AppResult<usize> {
    if !install_dir.join(crate::installation::manifest::PLAYER_EXE).exists() {
        return Err(AppError::NotFound(format!("{} is not a Roblox installation", install_dir.display())));
    }
    let file = install_dir.join("ClientSettings").join("ClientAppSettings.json");
    backups.create_from_file("client-app-settings", "before-apply", &file, 20)?;
    let mut out = Map::new();
    for (k, v) in flags {
        out.insert(k.clone(), validate::to_client_value(v));
    }
    store::write_json(&file, &Value::Object(out))?;
    Ok(flags.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn profile_lifecycle() {
        let mut s = FlagStore::default();
        assert_eq!(s.profiles.len(), 4);
        let id = s.profiles[3].id.clone();
        s.set_flag(&id, "FIntDebugForceMSAASamples", &json!("8"), None, true).unwrap();
        assert_eq!(s.get(&id).unwrap().flags["FIntDebugForceMSAASamples"], json!(8));
        assert!(s.set_flag(&id, "FIntX", &json!("nope"), None, true).is_err());
        assert!(s.set_flag(&id, "fintdebugforcemsaasamples", &json!(1), None, true).is_err(), "case-insensitive duplicate");
        s.set_flag(&id, "FIntFRMMaxGrassDistance", &json!(0), Some("FIntDebugForceMSAASamples"), true).unwrap();
        assert_eq!(s.get(&id).unwrap().flags.len(), 1);

        let dup = s.duplicate(&id).unwrap();
        assert_eq!(dup.name, "Custom (2)");
        assert!(s.rename(&dup.id, "graphics", None).is_err());
        s.rename(&dup.id, "Mine", Some("desc")).unwrap();
        let cmp = s.compare(&id, &s.profiles[1].id.clone()).unwrap();
        assert_eq!(cmp.only_left, vec!["FIntFRMMaxGrassDistance".to_string()]);
        assert_eq!(cmp.only_right.len(), 3);

        let (n, rej) = s.import(&id, r#"{"FFlagDebugSkyGray":"True","FIntBad":"x","bad name":1}"#, false, true).unwrap();
        assert_eq!(n, 1);
        assert_eq!(rej.len(), 2);
        assert!(s.import(&id, "[1,2]", false, true).is_err());
        let exported: BTreeMap<String, Value> = serde_json::from_str(&s.export(&id).unwrap()).unwrap();
        assert_eq!(exported["FFlagDebugSkyGray"], json!(true));
        s.reset(&id).unwrap();
        assert!(s.get(&id).unwrap().flags.is_empty());
        s.delete(&dup.id).unwrap();
        assert_eq!(s.profiles.len(), 4);
    }

    #[test]
    fn apply_writes_client_format_and_backs_up() {
        let d = tempfile::tempdir().unwrap();
        let inst = d.path().join("version-abc");
        std::fs::create_dir_all(inst.join("ClientSettings")).unwrap();
        std::fs::write(inst.join("RobloxPlayerBeta.exe"), b"").unwrap();
        std::fs::write(inst.join("ClientSettings/ClientAppSettings.json"), b"{\"Old\":\"1\"}").unwrap();
        let b = store::Backups::new(d.path().join("backups"));
        let mut flags = BTreeMap::new();
        flags.insert("FFlagDebugSkyGray".to_string(), json!(true));
        flags.insert("FIntDebugForceMSAASamples".to_string(), json!(4));
        assert_eq!(apply_to_install(&inst, &flags, &b).unwrap(), 2);
        let written: Value = serde_json::from_slice(&std::fs::read(inst.join("ClientSettings/ClientAppSettings.json")).unwrap()).unwrap();
        assert_eq!(written, json!({"FFlagDebugSkyGray":"True","FIntDebugForceMSAASamples":"4"}));
        assert_eq!(b.list("client-app-settings").unwrap().len(), 1);
        assert!(apply_to_install(d.path(), &flags, &b).is_err());
    }
}
