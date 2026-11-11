//! Application settings model. Every struct uses `#[serde(default)]` so that
//! fields added in later versions are filled with defaults when reading older files.

use serde::{Deserialize, Serialize};

pub const SETTINGS_SCHEMA_VERSION: u32 = 2;

/// Official Aeolus release feed (published by the GitHub release workflow).
pub const DEFAULT_UPDATE_FEED: &str = "https://github.com/aeolusrbx/Aeolus/releases/latest/download/aeolus-latest.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Animations {
    Enabled,
    Reduced,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    pub theme: Theme,
    pub accent: String,
    pub language: String,
    pub launch_at_startup: bool,
    pub start_minimized: bool,
    pub minimize_to_tray: bool,
    pub default_page: String,
    pub sidebar_collapsed: bool,
}
impl Default for General {
    fn default() -> Self {
        Self {
            theme: Theme::Dark,
            accent: "#FF861F".into(),
            language: "en".into(),
            launch_at_startup: false,
            start_minimized: false,
            minimize_to_tray: false,
            default_page: "home".into(),
            sidebar_collapsed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Appearance {
    pub animations: Animations,
    pub compact_navigation: bool,
    /// "plain" | "glow" | "grid"
    pub dashboard_background: String,
    /// "full" | "icon"
    pub logo_presentation: String,
    /// "solid" | "subtle"
    pub sidebar_style: String,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            animations: Animations::Enabled,
            compact_navigation: false,
            dashboard_background: "glow".into(),
            logo_presentation: "full".into(),
            sidebar_style: "solid".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Installation {
    /// Root folder for Aeolus-managed Roblox versions. `None` = default location.
    pub custom_install_root: Option<String>,
    pub scan_official_install: bool,
    pub scan_other_bootstrappers: bool,
    pub backup_before_destructive: bool,
    pub keep_backups: u32,
    pub check_updates_on_startup: bool,
    pub update_before_launch: bool,
    pub keep_download_cache: bool,
}
impl Default for Installation {
    fn default() -> Self {
        Self {
            custom_install_root: None,
            scan_official_install: true,
            scan_other_bootstrappers: true,
            backup_before_destructive: true,
            keep_backups: 10,
            check_updates_on_startup: true,
            update_before_launch: true,
            keep_download_cache: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Launch {
    pub default_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct FastFlagSettings {
    pub default_profile_id: Option<String>,
    pub backup_on_change: bool,
    pub max_backups: u32,
    pub warn_not_allowlisted: bool,
    pub block_invalid_values: bool,
}
impl Default for FastFlagSettings {
    fn default() -> Self {
        Self {
            default_profile_id: None,
            backup_on_change: true,
            max_backups: 25,
            warn_not_allowlisted: true,
            block_invalid_values: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PluginSettings {
    pub require_reapproval_on_change: bool,
    pub notify_on_errors: bool,
}
impl Default for PluginSettings {
    fn default() -> Self {
        Self { require_reapproval_on_change: true, notify_on_errors: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Notifications {
    pub native: bool,
    pub update_available: bool,
    pub update_completed: bool,
    pub update_failed: bool,
    pub version_changed: bool,
    pub launch_failed: bool,
    pub plugin_events: bool,
}
impl Default for Notifications {
    fn default() -> Self {
        Self {
            native: true,
            update_available: true,
            update_completed: true,
            update_failed: true,
            version_changed: true,
            launch_failed: true,
            plugin_events: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Discord {
    /// Show "Running Roblox with Aeolus" on your Discord profile while Roblox runs.
    pub rich_presence: bool,
    /// Include the launch profile's name in the presence.
    pub show_profile: bool,
}
impl Default for Discord {
    fn default() -> Self {
        Self { rich_presence: true, show_profile: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Privacy {
    pub log_retention_days: u32,
    pub include_logs_in_reports: bool,
}
impl Default for Privacy {
    fn default() -> Self {
        Self { log_retention_days: 14, include_logs_in_reports: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Advanced {
    pub developer_mode: bool,
    pub debug_logging: bool,
    /// Register Aeolus as the handler for `roblox-player:` links.
    pub handle_roblox_links: bool,
    /// URL of an Aeolus release feed (JSON). Empty = self-update checks disabled.
    pub self_update_feed: String,
    /// Check the release feed for a newer Aeolus when the app starts.
    pub check_aeolus_updates_on_startup: bool,
}
impl Default for Advanced {
    fn default() -> Self {
        Self {
            developer_mode: false,
            debug_logging: false,
            handle_roblox_links: false,
            self_update_feed: DEFAULT_UPDATE_FEED.into(),
            check_aeolus_updates_on_startup: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub schema_version: u32,
    pub general: General,
    pub appearance: Appearance,
    pub installation: Installation,
    pub launch: Launch,
    pub fast_flags: FastFlagSettings,
    pub plugins: PluginSettings,
    pub notifications: Notifications,
    pub discord: Discord,
    pub privacy: Privacy,
    pub advanced: Advanced,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            general: General::default(),
            appearance: Appearance::default(),
            installation: Installation::default(),
            launch: Launch::default(),
            fast_flags: FastFlagSettings::default(),
            plugins: PluginSettings::default(),
            notifications: Notifications::default(),
            discord: Discord::default(),
            privacy: Privacy::default(),
            advanced: Advanced::default(),
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl Settings {
    /// Schema validation beyond what serde enforces.
    pub fn validate(&self) -> Result<(), String> {
        if !is_hex_color(&self.general.accent) {
            return Err(format!("Accent color '{}' must be a hex color like #FF861F", self.general.accent));
        }
        const PAGES: [&str; 8] = ["home", "installations", "launch", "fastflags", "customization", "plugins", "diagnostics", "settings"];
        if !PAGES.contains(&self.general.default_page.as_str()) {
            return Err(format!("Unknown default page '{}'", self.general.default_page));
        }
        if !["plain", "glow", "grid"].contains(&self.appearance.dashboard_background.as_str()) {
            return Err("Unknown dashboard background".into());
        }
        if !["full", "icon"].contains(&self.appearance.logo_presentation.as_str()) {
            return Err("Unknown logo presentation".into());
        }
        if !["solid", "subtle"].contains(&self.appearance.sidebar_style.as_str()) {
            return Err("Unknown sidebar style".into());
        }
        if !(1..=100).contains(&self.installation.keep_backups) {
            return Err("Backups to keep must be between 1 and 100".into());
        }
        if !(1..=200).contains(&self.fast_flags.max_backups) {
            return Err("Fast Flag backups must be between 1 and 200".into());
        }
        if !(1..=365).contains(&self.privacy.log_retention_days) {
            return Err("Log retention must be between 1 and 365 days".into());
        }
        if let Some(root) = &self.installation.custom_install_root {
            let p = std::path::Path::new(root);
            if !p.is_absolute() {
                return Err("The installation directory must be an absolute path".into());
            }
        }
        let feed = self.advanced.self_update_feed.trim();
        if !feed.is_empty() && !feed.starts_with("https://") {
            return Err("The update feed must be an https:// URL".into());
        }
        Ok(())
    }
}
