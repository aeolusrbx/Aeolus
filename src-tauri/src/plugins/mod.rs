//! Plugin system (API v1).
//!
//! Plugins are *declarative*: a plugin is a folder containing `aeolus-plugin.json`.
//! Aeolus never executes plugin code. Plugins contribute data (Fast Flag presets,
//! configuration fields) and lifecycle hooks made of a restricted set of host
//! actions (log, notify). Every capability requires a declared permission that
//! the user approves; a changed manifest requires re-approval.

use crate::configuration::store;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const HOST_API_VERSION: u32 = 1;
pub const MANIFEST_FILE: &str = "aeolus-plugin.json";
pub const EXAMPLE_PLUGIN: &str = include_str!("../../../plugins/examples/graphics-presets/aeolus-plugin.json");

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    #[serde(rename = "log")]
    Log,
    #[serde(rename = "notifications")]
    Notifications,
    #[serde(rename = "fastflags.presets")]
    FastFlagPresets,
    #[serde(rename = "launch.hooks")]
    LaunchHooks,
}

impl Permission {
    pub fn describe(&self) -> &'static str {
        match self {
            Permission::Log => "Write entries to the Aeolus log",
            Permission::Notifications => "Show notifications",
            Permission::FastFlagPresets => "Offer Fast Flag presets you can import into a profile",
            Permission::LaunchHooks => "Run its hooks before and after Roblox launches",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum HookAction {
    Log { message: String },
    Notify { title: String, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Hooks {
    pub on_enable: Vec<HookAction>,
    pub on_disable: Vec<HookAction>,
    pub before_launch: Vec<HookAction>,
    pub after_launch: Vec<HookAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlagPreset {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub flags: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigField {
    pub key: String,
    pub label: String,
    /// "boolean" | "string" | "integer"
    #[serde(rename = "type")]
    pub kind: String,
    pub default: Value,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    pub id: String,
    #[serde(default)]
    pub min_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Contributes {
    pub fast_flag_presets: Vec<FlagPreset>,
    pub hooks: Hooks,
    pub configuration: Vec<ConfigField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub homepage: Option<String>,
    pub aeolus_api: u32,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    #[serde(default)]
    pub contributes: Contributes,
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut it = v.trim().split('.').map(|p| p.parse::<u64>().ok());
    Some((it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0)))
}

pub fn validate_manifest(m: &Manifest) -> Result<(), String> {
    let id_ok = (3..=80).contains(&m.id.len())
        && m.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
        && !m.id.starts_with('.')
        && !m.id.contains("..");
    if !id_ok {
        return Err("Plugin id must be 3-80 characters of lowercase letters, digits, '.' and '-'".into());
    }
    if m.name.trim().is_empty() || m.name.len() > 60 {
        return Err("Plugin name must be 1-60 characters".into());
    }
    if parse_version(&m.version).is_none() {
        return Err(format!("Invalid version '{}'", m.version));
    }
    if m.aeolus_api == 0 || m.aeolus_api > HOST_API_VERSION {
        return Err(format!("Requires Aeolus plugin API {} (this version supports {HOST_API_VERSION})", m.aeolus_api));
    }
    let h = &m.contributes.hooks;
    let all_hooks = h.on_enable.iter().chain(&h.on_disable).chain(&h.before_launch).chain(&h.after_launch);
    for a in all_hooks {
        let needed = match a {
            HookAction::Log { .. } => Permission::Log,
            HookAction::Notify { .. } => Permission::Notifications,
        };
        if !m.permissions.contains(&needed) {
            return Err(format!("A hook uses '{needed:?}' but the permission is not declared"));
        }
    }
    if (!h.before_launch.is_empty() || !h.after_launch.is_empty()) && !m.permissions.contains(&Permission::LaunchHooks) {
        return Err("Launch hooks require the 'launch.hooks' permission".into());
    }
    if !m.contributes.fast_flag_presets.is_empty() && !m.permissions.contains(&Permission::FastFlagPresets) {
        return Err("Fast Flag presets require the 'fastflags.presets' permission".into());
    }
    for preset in &m.contributes.fast_flag_presets {
        for (k, v) in &preset.flags {
            crate::fastflags::validate::valid_name(k)?;
            crate::fastflags::validate::coerce(k, v)?;
        }
    }
    for f in &m.contributes.configuration {
        if !["boolean", "string", "integer"].contains(&f.kind.as_str()) {
            return Err(format!("Configuration field '{}' has unsupported type '{}'", f.key, f.kind));
        }
        if !config_value_ok(&f.kind, &f.default) {
            return Err(format!("Default for '{}' does not match its type", f.key));
        }
    }
    Ok(())
}

fn config_value_ok(kind: &str, v: &Value) -> bool {
    match kind {
        "boolean" => v.is_boolean(),
        "integer" => v.is_i64(),
        "string" => v.as_str().map(|s| s.len() <= 500).unwrap_or(false),
        _ => false,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct PluginRecord {
    pub enabled: bool,
    pub approved_permissions: Vec<Permission>,
    pub approved_hash: Option<String>,
    pub config: Map<String, Value>,
    pub installed_at: Option<chrono::DateTime<chrono::Local>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct PluginStateFile {
    pub schema_version: u32,
    pub plugins: BTreeMap<String, PluginRecord>,
    pub example_seeded: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub manifest: Option<Manifest>,
    /// "enabled" | "disabled" | "needs-approval" | "changed" | "error"
    pub status: String,
    pub error: Option<String>,
    pub permissions: Vec<(Permission, String)>,
    pub dependencies_ok: bool,
    pub dependency_issues: Vec<String>,
    pub config: Map<String, Value>,
    pub path: String,
    pub last_updated: Option<chrono::DateTime<chrono::Local>>,
}

pub struct PluginHost {
    dir: PathBuf,
    state_path: PathBuf,
    pub state: PluginStateFile,
}

fn sha256_hex(b: &[u8]) -> String {
    use sha2::Digest;
    hex::encode(sha2::Sha256::digest(b))
}

impl PluginHost {
    pub fn load(dir: PathBuf, state_path: PathBuf) -> Self {
        let state = store::load_json_or_default::<PluginStateFile>(&state_path).map(|l| l.value).unwrap_or_default();
        Self { dir, state_path, state }
    }

    pub fn save(&self) -> AppResult<()> {
        store::write_json(&self.state_path, &self.state)
    }

    /// Put the bundled example plugin in place once (disabled, awaiting approval).
    pub fn seed_example(&mut self) -> AppResult<()> {
        if self.state.example_seeded {
            return Ok(());
        }
        let m: Manifest = serde_json::from_str(EXAMPLE_PLUGIN)?;
        let d = self.dir.join(&m.id);
        if !d.exists() {
            std::fs::create_dir_all(&d)?;
            std::fs::write(d.join(MANIFEST_FILE), EXAMPLE_PLUGIN)?;
        }
        self.state.example_seeded = true;
        self.save()
    }

    fn read_manifest(&self, id: &str) -> Result<(Manifest, String, PathBuf), String> {
        let path = crate::paths::safe_join(&self.dir, id).ok_or("invalid plugin id")?.join(MANIFEST_FILE);
        let bytes = std::fs::read(&path).map_err(|e| format!("cannot read manifest: {e}"))?;
        let m: Manifest = serde_json::from_slice(&bytes).map_err(|e| format!("manifest is not valid: {e}"))?;
        validate_manifest(&m)?;
        if m.id != id {
            return Err(format!("manifest id '{}' does not match folder '{id}'", m.id));
        }
        Ok((m, sha256_hex(&bytes), path))
    }

    fn ids(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(&self.dir)
            .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().to_string()).collect())
            .unwrap_or_default();
        v.sort();
        v
    }

    fn manifests(&self) -> BTreeMap<String, Manifest> {
        self.ids().into_iter().filter_map(|id| self.read_manifest(&id).ok().map(|(m, _, _)| (id, m))).collect()
    }

    fn dependency_issues(&self, m: &Manifest, all: &BTreeMap<String, Manifest>) -> Vec<String> {
        let mut issues = vec![];
        for d in &m.dependencies {
            match all.get(&d.id) {
                None => issues.push(format!("requires '{}' which is not installed", d.id)),
                Some(dep) => {
                    if let Some(min) = &d.min_version {
                        if parse_version(&dep.version) < parse_version(min) {
                            issues.push(format!("requires '{}' {min} or newer (installed {})", d.id, dep.version));
                        }
                    }
                    if !self.state.plugins.get(&d.id).map(|r| r.enabled).unwrap_or(false) {
                        issues.push(format!("requires '{}' to be enabled", d.id));
                    }
                }
            }
        }
        issues
    }

    pub fn list(&self, require_reapproval: bool) -> Vec<PluginInfo> {
        let all = self.manifests();
        self.ids()
            .into_iter()
            .map(|id| {
                let rec = self.state.plugins.get(&id).cloned().unwrap_or_default();
                let path = self.dir.join(&id).to_string_lossy().to_string();
                let last_updated = std::fs::metadata(self.dir.join(&id).join(MANIFEST_FILE))
                    .and_then(|m| m.modified())
                    .ok()
                    .map(chrono::DateTime::<chrono::Local>::from);
                match self.read_manifest(&id) {
                    Err(e) => PluginInfo {
                        id,
                        manifest: None,
                        status: "error".into(),
                        error: Some(e),
                        permissions: vec![],
                        dependencies_ok: false,
                        dependency_issues: vec![],
                        config: Map::new(),
                        path,
                        last_updated,
                    },
                    Ok((m, hash, _)) => {
                        let changed = rec.approved_hash.as_deref().map(|h| h != hash).unwrap_or(false);
                        let dep = self.dependency_issues(&m, &all);
                        let status = if changed && require_reapproval {
                            "changed"
                        } else if rec.approved_hash.is_none() {
                            "needs-approval"
                        } else if rec.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        };
                        let mut config = Map::new();
                        for f in &m.contributes.configuration {
                            config.insert(f.key.clone(), rec.config.get(&f.key).cloned().unwrap_or_else(|| f.default.clone()));
                        }
                        PluginInfo {
                            id,
                            permissions: m.permissions.iter().map(|p| (*p, p.describe().to_string())).collect(),
                            dependencies_ok: dep.is_empty(),
                            dependency_issues: dep,
                            manifest: Some(m),
                            status: status.into(),
                            error: rec.last_error.clone(),
                            config,
                            path,
                            last_updated,
                        }
                    }
                }
            })
            .collect()
    }

    /// Approve the plugin's declared permissions (pinning the manifest hash) and enable it.
    pub fn approve_and_enable(&mut self, id: &str) -> AppResult<Manifest> {
        let (m, hash, _) = self.read_manifest(id).map_err(AppError::Validation)?;
        let all = self.manifests();
        let issues = self.dependency_issues(&m, &all);
        if !issues.is_empty() {
            return Err(AppError::Validation(format!("Cannot enable {}: {}", m.name, issues.join("; "))));
        }
        let rec = self.state.plugins.entry(id.to_string()).or_default();
        rec.approved_permissions = m.permissions.clone();
        rec.approved_hash = Some(hash);
        rec.enabled = true;
        rec.last_error = None;
        rec.installed_at.get_or_insert_with(chrono::Local::now);
        self.save()?;
        Ok(m)
    }

    pub fn disable(&mut self, id: &str) -> AppResult<Option<Manifest>> {
        let rec = self.state.plugins.entry(id.to_string()).or_default();
        rec.enabled = false;
        self.save()?;
        Ok(self.read_manifest(id).ok().map(|(m, _, _)| m))
    }

    pub fn set_config(&mut self, id: &str, key: &str, value: Value) -> AppResult<()> {
        let (m, _, _) = self.read_manifest(id).map_err(AppError::Validation)?;
        let field = m.contributes.configuration.iter().find(|f| f.key == key).ok_or_else(|| AppError::Validation(format!("Unknown setting '{key}'")))?;
        if !config_value_ok(&field.kind, &value) {
            return Err(AppError::Validation(format!("'{}' expects a {}", field.label, field.kind)));
        }
        self.state.plugins.entry(id.to_string()).or_default().config.insert(key.into(), value);
        self.save()
    }

    /// Install from a folder containing aeolus-plugin.json. Only the manifest (and an
    /// optional README.md) is copied: plugins cannot ship executable files.
    pub fn install_from(&mut self, src: &Path) -> AppResult<Manifest> {
        let manifest_path = if src.is_file() { src.to_path_buf() } else { src.join(MANIFEST_FILE) };
        let bytes = std::fs::read(&manifest_path).map_err(|_| AppError::NotFound(format!("No {MANIFEST_FILE} found in the selected location")))?;
        let m: Manifest = serde_json::from_slice(&bytes).map_err(|e| AppError::Validation(format!("Invalid plugin manifest: {e}")))?;
        validate_manifest(&m).map_err(AppError::Validation)?;
        let dest = crate::paths::safe_join(&self.dir, &m.id).ok_or_else(|| AppError::Validation("invalid plugin id".into()))?;
        std::fs::create_dir_all(&dest)?;
        store::atomic_write(&dest.join(MANIFEST_FILE), &bytes)?;
        if let Some(parent) = manifest_path.parent() {
            if let Ok(readme) = std::fs::read(parent.join("README.md")) {
                std::fs::write(dest.join("README.md"), readme)?;
            }
        }
        // A reinstall with different contents must be re-approved.
        if let Some(rec) = self.state.plugins.get_mut(&m.id) {
            if rec.approved_hash.as_deref() != Some(&sha256_hex(&bytes)) {
                rec.enabled = false;
            }
        }
        self.save()?;
        Ok(m)
    }

    pub fn uninstall(&mut self, id: &str) -> AppResult<()> {
        let dir = crate::paths::safe_join(&self.dir, id).ok_or_else(|| AppError::Validation("invalid plugin id".into()))?;
        if !dir.join(MANIFEST_FILE).exists() && !dir.exists() {
            return Err(AppError::NotFound("Plugin not found".into()));
        }
        std::fs::remove_dir_all(&dir)?;
        self.state.plugins.remove(id);
        self.save()
    }

    /// Enabled, approved, unchanged plugins.
    pub fn active(&self) -> Vec<(Manifest, PluginRecord)> {
        self.ids()
            .into_iter()
            .filter_map(|id| {
                let rec = self.state.plugins.get(&id)?.clone();
                if !rec.enabled {
                    return None;
                }
                let (m, hash, _) = self.read_manifest(&id).ok()?;
                if rec.approved_hash.as_deref() != Some(&hash) {
                    return None;
                }
                Some((m, rec))
            })
            .collect()
    }

    pub fn record_error(&mut self, id: &str, err: &str) {
        self.state.plugins.entry(id.to_string()).or_default().last_error = Some(err.to_string());
        let _ = self.save();
    }
}

/// Expand `{{config.key}}` placeholders using the plugin's configuration.
pub fn render(template: &str, config: &Map<String, Value>, defaults: &[ConfigField]) -> String {
    let mut out = template.to_string();
    for f in defaults {
        let v = config.get(&f.key).unwrap_or(&f.default);
        let s = match v {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        out = out.replace(&format!("{{{{config.{}}}}}", f.key), &s);
    }
    out.chars().take(500).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_manifest_is_valid() {
        let m: Manifest = serde_json::from_str(EXAMPLE_PLUGIN).unwrap();
        validate_manifest(&m).unwrap();
    }

    #[test]
    fn permission_enforcement() {
        let mut m: Manifest = serde_json::from_str(EXAMPLE_PLUGIN).unwrap();
        m.permissions.retain(|p| *p != Permission::Notifications);
        assert!(validate_manifest(&m).is_err());
        let mut m2: Manifest = serde_json::from_str(EXAMPLE_PLUGIN).unwrap();
        m2.aeolus_api = 99;
        assert!(validate_manifest(&m2).is_err());
        let mut m3: Manifest = serde_json::from_str(EXAMPLE_PLUGIN).unwrap();
        m3.id = "../evil".into();
        assert!(validate_manifest(&m3).is_err());
    }

    #[test]
    fn lifecycle_integrity_and_dependencies() {
        let d = tempfile::tempdir().unwrap();
        let mut h = PluginHost::load(d.path().join("plugins"), d.path().join("plugins.json"));
        std::fs::create_dir_all(d.path().join("plugins")).unwrap();
        h.seed_example().unwrap();
        let list = h.list(true);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].status, "needs-approval");
        assert!(h.active().is_empty(), "never active without approval");
        let id = list[0].id.clone();
        h.approve_and_enable(&id).unwrap();
        assert_eq!(h.active().len(), 1);

        // tamper with the manifest -> no longer active, shows as changed
        let p = d.path().join("plugins").join(&id).join(MANIFEST_FILE);
        let text = std::fs::read_to_string(&p).unwrap().replace("\"version\": \"1.0.0\"", "\"version\": \"1.0.1\"");
        std::fs::write(&p, text).unwrap();
        assert!(h.active().is_empty());
        assert_eq!(h.list(true)[0].status, "changed");

        // config validation
        assert!(h.set_config(&id, "launchMessage", Value::Bool(true)).is_err());
        h.set_config(&id, "launchMessage", Value::String("hi".into())).unwrap();

        // dependency
        let dep_dir = d.path().join("src");
        std::fs::create_dir_all(&dep_dir).unwrap();
        std::fs::write(dep_dir.join(MANIFEST_FILE), r#"{"id":"dev.test.child","name":"Child","version":"1.0.0","aeolusApi":1,"dependencies":[{"id":"dev.test.missing"}]}"#).unwrap();
        h.install_from(&dep_dir).unwrap();
        let err = h.approve_and_enable("dev.test.child").unwrap_err();
        assert!(err.to_string().contains("not installed"));
        h.uninstall("dev.test.child").unwrap();
        assert_eq!(h.list(true).len(), 1);
    }

    #[test]
    fn renders_templates() {
        let f = vec![ConfigField { key: "name".into(), label: "N".into(), kind: "string".into(), default: Value::String("world".into()), description: String::new() }];
        assert_eq!(render("hello {{config.name}}", &Map::new(), &f), "hello world");
    }
}
