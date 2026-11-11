//! Shared application state.

use crate::configuration::{history::History, settings::Settings, store, store::Backups};
use crate::diagnostics::{activity::Activity, logging::Logger};
use crate::error::{AppError, AppResult};
use crate::fastflags::FlagStore;
use crate::installation::{progress::CancelToken, InstallState};
use crate::launcher::LaunchRecord;
use crate::paths::Paths;
use crate::plugins::PluginHost;
use crate::profiles::ProfileStore;
use crate::updates::UpdateRecord;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};

pub struct AppState {
    pub paths: Paths,
    pub http: reqwest::Client,
    pub logger: Arc<Logger>,
    pub activity: Activity,
    pub backups: Backups,
    pub settings: RwLock<Settings>,
    pub install: Mutex<InstallState>,
    pub flags: Mutex<FlagStore>,
    pub profiles: Mutex<ProfileStore>,
    pub plugins: Mutex<PluginHost>,
    pub launches: History<LaunchRecord>,
    pub updates: History<UpdateRecord>,
    pub operation: Mutex<Option<(String, CancelToken)>>,
    pub started_minimized: AtomicBool,
    pub startup_notes: Mutex<Vec<String>>,
    pub presence: crate::system::discord::Presence,
    pub skies: crate::skies::SkyLibrary,
    /// The Roblox session started by Aeolus that is currently running, if any.
    pub session: Mutex<Option<RobloxSession>>,
}

#[derive(Debug, Clone)]
pub struct RobloxSession {
    pub pid: u32,
    pub started_at: chrono::DateTime<chrono::Local>,
    pub profile: Option<String>,
}

pub const SETTINGS_FILE: &str = "settings.json";
pub const INSTALL_FILE: &str = "installation.json";
pub const FLAGS_FILE: &str = "fastflags.json";
pub const PROFILES_FILE: &str = "profiles.json";
pub const PLUGINS_FILE: &str = "plugins.json";
pub const ACTIVITY_FILE: &str = "activity.json";
pub const LAUNCHES_FILE: &str = "launch-history.json";
pub const UPDATES_FILE: &str = "update-history.json";
pub const PROTOCOL_BACKUP_FILE: &str = "protocol-backup.json";

/// Releases the operation slot when dropped.
pub struct OperationGuard<'a>(&'a Mutex<Option<(String, CancelToken)>>);
impl Drop for OperationGuard<'_> {
    fn drop(&mut self) {
        *self.0.lock().unwrap_or_else(|p| p.into_inner()) = None;
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

impl AppState {
    pub fn init(paths: Paths) -> AppResult<Self> {
        paths.ensure()?;
        let logger = Arc::new(Logger::new(paths.logs()));
        let backups = Backups::new(paths.backups());
        let mut notes = vec![];
        let (settings, n) = crate::configuration::load_settings(&paths.file(SETTINGS_FILE), &backups)?;
        notes.extend(n);
        logger.set_debug(settings.advanced.debug_logging);

        let install = Self::load_doc::<InstallState>(&paths, INSTALL_FILE, &mut notes)?;
        let flags = Self::load_doc::<FlagStore>(&paths, FLAGS_FILE, &mut notes)?;
        let profiles = Self::load_doc::<ProfileStore>(&paths, PROFILES_FILE, &mut notes)?;
        let mut plugins = PluginHost::load(paths.plugins(), paths.file(PLUGINS_FILE));
        if let Err(e) = plugins.seed_example() {
            notes.push(format!("Could not install the example plugin: {e}"));
        }
        let http = reqwest::Client::builder()
            .user_agent(concat!("Aeolus/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(std::time::Duration::from_secs(15))
            .read_timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| AppError::Other(e.to_string()))?;
        for n in &notes {
            logger.warn("config", n);
        }
        Ok(Self {
            http,
            activity: Activity::load(paths.file(ACTIVITY_FILE)),
            launches: History::load(paths.file(LAUNCHES_FILE), 200),
            updates: History::load(paths.file(UPDATES_FILE), 200),
            backups,
            settings: RwLock::new(settings),
            install: Mutex::new(install),
            flags: Mutex::new(flags),
            profiles: Mutex::new(profiles),
            plugins: Mutex::new(plugins),
            operation: Mutex::new(None),
            started_minimized: AtomicBool::new(false),
            startup_notes: Mutex::new(notes),
            presence: crate::system::discord::Presence::start(logger.clone()),
            skies: crate::skies::SkyLibrary::new(paths.skies()),
            session: Mutex::new(None),
            logger,
            paths,
        })
    }

    fn load_doc<T: serde::de::DeserializeOwned + serde::Serialize + Default>(paths: &Paths, file: &str, notes: &mut Vec<String>) -> AppResult<T> {
        let p = paths.file(file);
        let existed = p.exists();
        let l = store::load_json_or_default::<T>(&p)?;
        if let Some(aside) = l.recovered_from {
            notes.push(format!("{file} was unreadable; it was preserved as {} and defaults were restored", aside.display()));
        }
        if !existed || notes.iter().any(|n| n.starts_with(file)) {
            store::write_json(&p, &l.value)?;
        }
        Ok(l.value)
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn managed_root(&self) -> std::path::PathBuf {
        match self.settings().installation.custom_install_root {
            Some(r) if !r.trim().is_empty() => std::path::PathBuf::from(r),
            _ => self.paths.default_versions_root(),
        }
    }

    pub fn begin_operation(&self, name: &str) -> AppResult<(CancelToken, OperationGuard<'_>)> {
        let mut g = lock(&self.operation);
        if let Some((running, _)) = g.as_ref() {
            return Err(AppError::Busy(running.clone()));
        }
        let t = CancelToken::new();
        *g = Some((name.to_string(), t.clone()));
        Ok((t, OperationGuard(&self.operation)))
    }

    pub fn cancel_operation(&self) -> bool {
        match lock(&self.operation).as_ref() {
            Some((_, t)) => {
                t.cancel();
                true
            }
            None => false,
        }
    }

    pub fn current_operation(&self) -> Option<String> {
        lock(&self.operation).as_ref().map(|(n, _)| n.clone())
    }

    pub fn install_state(&self) -> InstallState {
        lock(&self.install).clone()
    }
    pub fn update_install<R>(&self, f: impl FnOnce(&mut InstallState) -> R) -> AppResult<R> {
        let mut g = lock(&self.install);
        let r = f(&mut g);
        store::write_json(&self.paths.file(INSTALL_FILE), &*g)?;
        Ok(r)
    }

    pub fn flags_snapshot(&self) -> FlagStore {
        lock(&self.flags).clone()
    }
    /// Mutate the flag store; backs up the previous version first if enabled.
    pub fn update_flags<R>(&self, label: &str, f: impl FnOnce(&mut FlagStore) -> AppResult<R>) -> AppResult<R> {
        let s = self.settings();
        let mut g = lock(&self.flags);
        let before = serde_json::to_vec_pretty(&*g)?;
        let mut working = g.clone();
        let r = f(&mut working)?;
        if s.fast_flags.backup_on_change {
            self.backups.create("fastflags", label, &before, s.fast_flags.max_backups as usize)?;
        }
        store::write_json(&self.paths.file(FLAGS_FILE), &working)?;
        *g = working;
        Ok(r)
    }

    pub fn profiles_snapshot(&self) -> ProfileStore {
        lock(&self.profiles).clone()
    }
    pub fn update_profiles<R>(&self, f: impl FnOnce(&mut ProfileStore) -> AppResult<R>) -> AppResult<R> {
        let mut g = lock(&self.profiles);
        let mut working = g.clone();
        let r = f(&mut working)?;
        store::write_json(&self.paths.file(PROFILES_FILE), &working)?;
        *g = working;
        Ok(r)
    }

    pub fn plugins(&self) -> std::sync::MutexGuard<'_, PluginHost> {
        lock(&self.plugins)
    }

    pub fn save_settings(&self, new: Settings) -> AppResult<()> {
        new.validate().map_err(AppError::Validation)?;
        let path = self.paths.file(SETTINGS_FILE);
        let keep = new.installation.keep_backups as usize;
        self.backups.create_from_file("settings", "before-change", &path, keep)?;
        store::write_json(&path, &new)?;
        self.logger.set_debug(new.advanced.debug_logging);
        *self.settings.write().unwrap_or_else(|p| p.into_inner()) = new;
        Ok(())
    }

    /// Back up all user configuration before a destructive operation.
    pub fn backup_user_config(&self, label: &str) -> AppResult<usize> {
        let keep = self.settings().installation.keep_backups as usize;
        let mut n = 0;
        for f in [SETTINGS_FILE, FLAGS_FILE, PROFILES_FILE, PLUGINS_FILE, INSTALL_FILE] {
            if self.backups.create_from_file(&format!("config-{}", f.trim_end_matches(".json")), label, &self.paths.file(f), keep)?.is_some() {
                n += 1;
            }
        }
        Ok(n)
    }
}
