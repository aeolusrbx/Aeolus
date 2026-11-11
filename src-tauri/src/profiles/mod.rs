//! Unified profiles. A profile bundles launch preferences, a reference to a
//! Fast Flag profile, and presentation (icon, accent). Profiles *reference*
//! Fast Flag profiles rather than copying them, so switching profiles never
//! overwrites unrelated configuration.

use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

pub const ICONS: &[&str] = &["wind", "gamepad-2", "zap", "sparkles", "monitor", "rocket", "flame", "shield", "star", "user"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AfterLaunch {
    Nothing,
    Minimize,
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct LaunchPrefs {
    /// Write the referenced Fast Flag profile before launching.
    pub apply_fast_flags: bool,
    /// Install a pending Roblox update before launching.
    pub update_before_launch: bool,
    /// Run a quick (size-based) integrity check before launching.
    pub verify_before_launch: bool,
    /// Apply your sounds, cursors and fonts from the Mods page.
    pub apply_mods: bool,
    pub after_launch: AfterLaunch,
}
impl Default for LaunchPrefs {
    fn default() -> Self {
        Self { apply_fast_flags: true, update_before_launch: true, verify_before_launch: false, apply_mods: true, after_launch: AfterLaunch::Nothing }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default)]
    pub fast_flag_profile_id: Option<String>,
    /// Sky for this profile: None = use the global sky, "default" = Roblox's sky, else a pack id.
    #[serde(default)]
    pub sky: Option<String>,
    #[serde(default)]
    pub launch: LaunchPrefs,
    pub created_at: chrono::DateTime<chrono::Local>,
    #[serde(default)]
    pub last_used_at: Option<chrono::DateTime<chrono::Local>>,
}
fn default_icon() -> String {
    "wind".into()
}
fn default_accent() -> String {
    "#FF861F".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ProfileStore {
    pub schema_version: u32,
    pub profiles: Vec<Profile>,
    pub default_id: Option<String>,
    pub active_id: Option<String>,
}

impl Default for ProfileStore {
    fn default() -> Self {
        let p = Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Default".into(),
            description: "Standard launch".into(),
            icon: default_icon(),
            accent: default_accent(),
            fast_flag_profile_id: None,
            sky: None,
            launch: LaunchPrefs::default(),
            created_at: chrono::Local::now(),
            last_used_at: None,
        };
        Self { schema_version: 1, default_id: Some(p.id.clone()), active_id: Some(p.id.clone()), profiles: vec![p] }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_icon")]
    pub icon: String,
    #[serde(default = "default_accent")]
    pub accent: String,
    #[serde(default)]
    pub fast_flag_profile_id: Option<String>,
    #[serde(default)]
    pub sky: Option<String>,
    #[serde(default)]
    pub launch: LaunchPrefs,
}

impl ProfileStore {
    pub fn get(&self, id: &str) -> AppResult<&Profile> {
        self.profiles.iter().find(|p| p.id == id).ok_or_else(|| AppError::NotFound("Profile not found".into()))
    }

    /// The profile used for launching: active -> default -> first.
    pub fn current(&self) -> Option<&Profile> {
        self.active_id
            .as_deref()
            .and_then(|id| self.get(id).ok())
            .or_else(|| self.default_id.as_deref().and_then(|id| self.get(id).ok()))
            .or_else(|| self.profiles.first())
    }

    fn validate(&self, input: &ProfileInput, except: Option<&str>) -> AppResult<()> {
        let n = input.name.trim();
        if n.is_empty() || n.len() > 60 {
            return Err(AppError::Validation("Profile names must be 1-60 characters".into()));
        }
        if self.profiles.iter().any(|p| Some(p.id.as_str()) != except && p.name.eq_ignore_ascii_case(n)) {
            return Err(AppError::Validation(format!("A profile named '{n}' already exists")));
        }
        if !ICONS.contains(&input.icon.as_str()) {
            return Err(AppError::Validation("Unknown icon".into()));
        }
        let a = &input.accent;
        if !(a.len() == 7 && a.starts_with('#') && a[1..].chars().all(|c| c.is_ascii_hexdigit())) {
            return Err(AppError::Validation("Accent must be a hex color like #FF861F".into()));
        }
        if input.description.len() > 300 {
            return Err(AppError::Validation("Description is too long".into()));
        }
        Ok(())
    }

    pub fn create(&mut self, input: ProfileInput) -> AppResult<Profile> {
        self.validate(&input, None)?;
        let p = Profile {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name.trim().into(),
            description: input.description,
            icon: input.icon,
            accent: input.accent,
            fast_flag_profile_id: input.fast_flag_profile_id,
            sky: input.sky,
            launch: input.launch,
            created_at: chrono::Local::now(),
            last_used_at: None,
        };
        self.profiles.push(p.clone());
        Ok(p)
    }

    pub fn update(&mut self, id: &str, input: ProfileInput) -> AppResult<Profile> {
        self.validate(&input, Some(id))?;
        let p = self.profiles.iter_mut().find(|p| p.id == id).ok_or_else(|| AppError::NotFound("Profile not found".into()))?;
        p.name = input.name.trim().into();
        p.description = input.description;
        p.icon = input.icon;
        p.accent = input.accent;
        p.fast_flag_profile_id = input.fast_flag_profile_id;
        p.sky = input.sky;
        p.launch = input.launch;
        Ok(p.clone())
    }

    pub fn duplicate(&mut self, id: &str) -> AppResult<Profile> {
        let src = self.get(id)?.clone();
        let mut n = 2;
        let name = loop {
            let c = format!("{} ({n})", src.name);
            if !self.profiles.iter().any(|p| p.name.eq_ignore_ascii_case(&c)) {
                break c;
            }
            n += 1;
        };
        self.create(ProfileInput {
            name,
            description: src.description,
            icon: src.icon,
            accent: src.accent,
            fast_flag_profile_id: src.fast_flag_profile_id,
            sky: src.sky,
            launch: src.launch,
        })
    }

    pub fn delete(&mut self, id: &str) -> AppResult<Profile> {
        if self.profiles.len() <= 1 {
            return Err(AppError::Validation("At least one profile must remain".into()));
        }
        let idx = self.profiles.iter().position(|p| p.id == id).ok_or_else(|| AppError::NotFound("Profile not found".into()))?;
        let removed = self.profiles.remove(idx);
        let first = self.profiles[0].id.clone();
        if self.default_id.as_deref() == Some(id) {
            self.default_id = Some(first.clone());
        }
        if self.active_id.as_deref() == Some(id) {
            self.active_id = self.default_id.clone();
        }
        Ok(removed)
    }

    pub fn set_default(&mut self, id: &str) -> AppResult<()> {
        self.get(id)?;
        self.default_id = Some(id.into());
        Ok(())
    }

    pub fn set_active(&mut self, id: &str) -> AppResult<()> {
        self.get(id)?;
        self.active_id = Some(id.into());
        Ok(())
    }

    pub fn touch(&mut self, id: &str) {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
            p.last_used_at = Some(chrono::Local::now());
        }
    }

    /// Clear references to a deleted Fast Flag profile.
    pub fn detach_flag_profile(&mut self, flag_id: &str) -> usize {
        let mut n = 0;
        for p in &mut self.profiles {
            if p.fast_flag_profile_id.as_deref() == Some(flag_id) {
                p.fast_flag_profile_id = None;
                n += 1;
            }
        }
        n
    }

    /// Clear references to a deleted sky pack.
    pub fn detach_sky(&mut self, sky_id: &str) -> usize {
        let mut n = 0;
        for p in &mut self.profiles {
            if p.sky.as_deref() == Some(sky_id) {
                p.sky = None;
                n += 1;
            }
        }
        n
    }

    pub fn export(&self, id: &str) -> AppResult<String> {
        let p = self.get(id)?;
        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "aeolusProfile": 1,
            "name": p.name,
            "description": p.description,
            "icon": p.icon,
            "accent": p.accent,
            "launch": p.launch,
        }))?)
    }

    /// Import a profile exported by Aeolus. Fast Flag references are not carried over
    /// because they point at another machine's data.
    pub fn import(&mut self, json: &str) -> AppResult<Profile> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Exported {
            aeolus_profile: u32,
            name: String,
            #[serde(default)]
            description: String,
            #[serde(default = "default_icon")]
            icon: String,
            #[serde(default = "default_accent")]
            accent: String,
            #[serde(default)]
            launch: LaunchPrefs,
        }
        let e: Exported = serde_json::from_str(json).map_err(|e| AppError::Validation(format!("Not an Aeolus profile file: {e}")))?;
        if e.aeolus_profile != 1 {
            return Err(AppError::Validation("Unsupported profile file version".into()));
        }
        let mut name = e.name.trim().to_string();
        if self.profiles.iter().any(|p| p.name.eq_ignore_ascii_case(&name)) {
            name = format!("{name} (imported)");
        }
        self.create(ProfileInput { name, description: e.description, icon: e.icon, accent: e.accent, fast_flag_profile_id: None, sky: None, launch: e.launch })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str) -> ProfileInput {
        ProfileInput { name: name.into(), description: String::new(), icon: "zap".into(), accent: "#123456".into(), fast_flag_profile_id: Some("ff1".into()), sky: None, launch: LaunchPrefs::default() }
    }

    #[test]
    fn crud_default_and_export() {
        let mut s = ProfileStore::default();
        let a = s.create(input("Competitive")).unwrap();
        assert!(s.create(input("competitive")).is_err());
        let mut bad = input("X");
        bad.icon = "nope".into();
        assert!(s.create(bad).is_err());
        let d = s.duplicate(&a.id).unwrap();
        assert_eq!(d.name, "Competitive (2)");
        s.set_default(&a.id).unwrap();
        s.set_active(&a.id).unwrap();
        assert_eq!(s.current().unwrap().id, a.id);
        s.delete(&a.id).unwrap();
        assert_ne!(s.default_id.as_deref(), Some(a.id.as_str()));
        assert!(s.current().is_some());
        assert_eq!(s.detach_flag_profile("ff1"), 1);

        let json = s.export(&d.id).unwrap();
        let imported = s.import(&json).unwrap();
        assert_eq!(imported.name, "Competitive (2) (imported)");
        assert!(imported.fast_flag_profile_id.is_none());
        assert!(s.import("{}").is_err());
        let only = ProfileStore::default();
        let mut only2 = only.clone();
        assert!(only2.delete(&only.profiles[0].id).is_err());
    }
}
