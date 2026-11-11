//! High-level operations shared by commands, the tray menu and link handling.

use crate::diagnostics::activity::ActivityKind;
use crate::error::{AppError, AppResult};
use crate::installation::{self, manifest, progress::Progress, InstallJob, InstallOutcome};
use crate::launcher::{self, LaunchRecord, LaunchResult};
use crate::plugins::{HookAction, Permission};
use crate::profiles::AfterLaunch;
use crate::state::AppState;
use crate::updates::{UpdateRecord, UpdateResult, UpdateStatus};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager};

pub const EV_PROGRESS: &str = "aeolus://progress";
pub const EV_CHANGED: &str = "aeolus://changed";
pub const EV_NAVIGATE: &str = "aeolus://navigate";
pub const EV_TOAST: &str = "aeolus://toast";

pub fn changed(app: &AppHandle, area: &str) {
    let _ = app.emit(EV_CHANGED, area);
}

/// In-app toast (always) + native Windows notification (if enabled).
pub fn notify(app: &AppHandle, level: &str, title: &str, body: &str, native: bool) {
    let _ = app.emit(EV_TOAST, serde_json::json!({ "level": level, "title": title, "description": body }));
    let state = app.state::<AppState>();
    if native && state.settings().notifications.native {
        use tauri_plugin_notification::NotificationExt;
        let _ = app.notification().builder().title(title).body(body).show();
    }
}

pub async fn check_updates(app: &AppHandle, notify_user: bool) -> AppResult<UpdateStatus> {
    let state = app.state::<AppState>();
    let res = installation::fetch_latest(&state.http, manifest::CLIENT_VERSION_URL).await;
    let previous_latest = state.install_state().last_check.latest.clone();
    match res {
        Ok(latest) => {
            state.update_install(|s| {
                s.last_check.checked_at = Some(chrono::Local::now());
                s.last_check.latest = Some(latest.clone());
                s.last_check.error = None;
            })?;
            state.logger.info("updates", format!("Latest Roblox version is {} ({})", latest.version, latest.version_guid));
            let st = crate::updates::status(&state.install_state());
            let n = state.settings().notifications;
            if st.update_available && notify_user && n.update_available && previous_latest.as_ref() != Some(&latest) {
                notify(app, "info", "Roblox update available", &format!("Version {} is ready to install.", latest.version), true);
            }
            if previous_latest.as_ref().map(|p| p.version_guid != latest.version_guid).unwrap_or(false) && n.version_changed && notify_user {
                state.activity.record(ActivityKind::Update, "Roblox released a new version", Some(latest.version.clone()));
            }
            changed(app, "install");
            Ok(st)
        }
        Err(e) => {
            state.update_install(|s| {
                s.last_check.checked_at = Some(chrono::Local::now());
                s.last_check.error = Some(e.to_string());
            })?;
            state.logger.warn("updates", format!("Update check failed: {e}"));
            changed(app, "install");
            Err(e)
        }
    }
}

/// kind: "install" | "update" | "repair" | "reinstall"
pub async fn run_install(app: &AppHandle, kind: &str) -> AppResult<InstallOutcome> {
    let state = app.state::<AppState>();
    let label = match kind {
        "install" => "Installing Roblox",
        "update" => "Updating Roblox",
        "repair" => "Repairing Roblox",
        "reinstall" => "Reinstalling Roblox",
        _ => return Err(AppError::Validation(format!("unknown operation {kind}"))),
    };
    let (cancel, _guard) = state.begin_operation(label)?;
    let started = std::time::Instant::now();
    let settings = state.settings();
    let previous = state.install_state().active.clone();
    state.logger.info("install", format!("{label} started"));
    changed(app, "operation");

    if matches!(kind, "reinstall" | "repair") && settings.installation.backup_before_destructive {
        let n = state.backup_user_config(&format!("before-{kind}"))?;
        state.activity.record(ActivityKind::Backup, format!("Configuration backed up before {kind}"), Some(format!("{n} files")));
    }
    if kind == "reinstall" {
        installation::clear_cache(&state.paths.downloads())?;
    }

    let target = if kind == "repair" {
        match &previous {
            Some(p) => manifest::ClientVersion { version: p.version.clone(), version_guid: p.version_guid.clone() },
            None => return Err(AppError::NotFound("There is no Aeolus-managed installation to repair".into())),
        }
    } else {
        let emit_app = app.clone();
        let _ = emit_app.emit(EV_PROGRESS, Progress { operation: label.into(), stage: "Checking latest version".into(), ..Default::default() });
        let latest = installation::fetch_latest(&state.http, manifest::CLIENT_VERSION_URL).await?;
        state.update_install(|s| {
            s.last_check.checked_at = Some(chrono::Local::now());
            s.last_check.latest = Some(latest.clone());
            s.last_check.error = None;
        })?;
        latest
    };

    let emitter = app.clone();
    let job = InstallJob {
        http: &state.http,
        hosts: manifest::DEPLOY_HOSTS.iter().map(|s| s.to_string()).collect(),
        root: state.managed_root(),
        cache_dir: state.paths.downloads(),
        keep_cache: settings.installation.keep_download_cache,
        cancel: cancel.clone(),
        sink: Arc::new(move |p: Progress| {
            let _ = emitter.emit(EV_PROGRESS, p);
        }),
        operation: label.into(),
    };
    let result = installation::install(&job, &target, previous.clone()).await;
    let elapsed = started.elapsed().as_secs_f64();
    let from_version = previous.as_ref().map(|p| p.version.clone());
    let notif = settings.notifications.clone();
    match result {
        Ok(outcome) => {
            state.update_install(|s| s.active = Some(outcome.installed.clone()))?;
            state.updates.push(UpdateRecord {
                id: uuid::Uuid::new_v4().to_string(),
                kind: kind.into(),
                from_version,
                to_version: outcome.installed.version.clone(),
                at: chrono::Local::now(),
                result: UpdateResult::Success,
                error: None,
                duration_seconds: elapsed,
            });
            for w in &outcome.warnings {
                state.logger.warn("install", w);
            }
            state.logger.info("install", format!("{label} finished: {} in {elapsed:.1}s", outcome.installed.version));
            state.activity.record(
                if kind == "update" { ActivityKind::Update } else { ActivityKind::Install },
                format!("{} {}", match kind { "update" => "Updated to", "repair" => "Repaired", "reinstall" => "Reinstalled", _ => "Installed" }, outcome.installed.version),
                Some(format!("{} files", outcome.installed.file_count)),
            );
            if notif.update_completed {
                notify(app, "success", &format!("{label} complete"), &format!("Roblox {} is ready.", outcome.installed.version), true);
            }
            let _ = app.emit(EV_PROGRESS, Progress { operation: label.into(), stage: "Completed".into(), done: true, ..Default::default() });
            changed(app, "install");
            Ok(outcome)
        }
        Err(e) => {
            let cancelled = matches!(e, AppError::Cancelled);
            state.updates.push(UpdateRecord {
                id: uuid::Uuid::new_v4().to_string(),
                kind: kind.into(),
                from_version,
                to_version: target.version.clone(),
                at: chrono::Local::now(),
                result: if cancelled { UpdateResult::Cancelled } else { UpdateResult::Failed },
                error: Some(e.to_string()),
                duration_seconds: elapsed,
            });
            if cancelled {
                state.logger.info("install", format!("{label} cancelled; completed downloads were kept"));
            } else {
                state.logger.error("install", format!("{label} failed: {e}"));
                state.activity.record(ActivityKind::Error, format!("{label} failed"), Some(e.to_string()));
                if notif.update_failed {
                    notify(app, "error", &format!("{label} failed"), &e.to_string(), true);
                }
            }
            let _ = app.emit(EV_PROGRESS, Progress { operation: label.into(), stage: if cancelled { "Cancelled".into() } else { "Failed".into() }, done: true, detail: Some(e.to_string()), ..Default::default() });
            changed(app, "install");
            Err(e)
        }
    }
}

pub fn run_hooks(app: &AppHandle, which: &str, only: Option<&str>) {
    let state = app.state::<AppState>();
    let active = state.plugins().active();
    for (m, rec) in active {
        if let Some(id) = only {
            if m.id != id {
                continue;
            }
        }
        let h = &m.contributes.hooks;
        let (actions, needs_launch) = match which {
            "onEnable" => (&h.on_enable, false),
            "onDisable" => (&h.on_disable, false),
            "beforeLaunch" => (&h.before_launch, true),
            "afterLaunch" => (&h.after_launch, true),
            _ => continue,
        };
        if needs_launch && !rec.approved_permissions.contains(&Permission::LaunchHooks) {
            continue;
        }
        for a in actions {
            match a {
                HookAction::Log { message } if rec.approved_permissions.contains(&Permission::Log) => {
                    state.logger.info(&format!("plugin:{}", m.id), crate::plugins::render(message, &rec.config, &m.contributes.configuration));
                }
                HookAction::Notify { title, message } if rec.approved_permissions.contains(&Permission::Notifications) => {
                    let t = crate::plugins::render(title, &rec.config, &m.contributes.configuration);
                    let b = crate::plugins::render(message, &rec.config, &m.contributes.configuration);
                    notify(app, "info", &t, &b, state.settings().notifications.plugin_events);
                }
                _ => {
                    let msg = format!("hook action blocked: permission not approved ({which})");
                    state.logger.warn(&format!("plugin:{}", m.id), &msg);
                    state.plugins().record_error(&m.id, &msg);
                }
            }
        }
    }
}

pub struct LaunchTarget {
    pub dir: std::path::PathBuf,
    pub version: Option<String>,
    pub managed: bool,
}

pub fn resolve_target(state: &AppState) -> AppResult<LaunchTarget> {
    let st = state.install_state();
    if let Some(ext) = &st.preferred_external {
        let p = std::path::PathBuf::from(ext);
        if launcher::player_exe(&p).is_file() {
            return Ok(LaunchTarget { version: p.file_name().map(|n| n.to_string_lossy().to_string()), dir: p, managed: false });
        }
        state.logger.warn("launch", "The selected external installation no longer exists; falling back to the Aeolus installation");
    }
    match st.active {
        Some(a) => Ok(LaunchTarget { dir: a.path.into(), version: Some(a.version), managed: true }),
        None => Err(AppError::NotFound("Roblox is not installed yet. Install it from the Installations page.".into())),
    }
}

pub async fn launch(app: &AppHandle, link: Option<String>, profile_id: Option<String>) -> AppResult<LaunchRecord> {
    let state = app.state::<AppState>();
    let profiles = state.profiles_snapshot();
    let profile = match &profile_id {
        Some(id) => Some(profiles.get(id)?.clone()),
        None => profiles.current().cloned(),
    };
    let prefs = profile.as_ref().map(|p| p.launch.clone()).unwrap_or_default();
    let flags_store = state.flags_snapshot();
    let flag_profile = profile
        .as_ref()
        .and_then(|p| p.fast_flag_profile_id.clone())
        .or(state.settings().fast_flags.default_profile_id)
        .and_then(|id| flags_store.get(&id).ok().cloned());
    let mode = if link.is_some() { "link" } else { "app" };
    state.logger.info("launch", format!("Launch requested ({mode}) with profile {}", profile.as_ref().map(|p| p.name.as_str()).unwrap_or("none")));

    let result: AppResult<(u32, Option<std::process::Child>, LaunchTarget)> = async {
        let args = launcher::build_args(link.as_deref())?;
        let mut target = resolve_target(&state)?;
        if target.managed && prefs.update_before_launch && state.settings().installation.update_before_launch {
            match check_updates(app, false).await {
                Ok(s) if s.update_available => {
                    run_install(app, "update").await?;
                    target = resolve_target(&state)?;
                }
                Ok(_) => {}
                Err(e) => state.logger.warn("launch", format!("Could not check for updates, launching installed version: {e}")),
            }
        }
        if prefs.verify_before_launch && target.managed {
            let r = crate::installation::deploy::verify(&target.dir, false, &Default::default(), &|_, _| {})?;
            if !r.ok() {
                return Err(AppError::Integrity(format!("{} missing and {} damaged files", r.missing.len(), r.modified.len())));
            }
        }
        if prefs.apply_fast_flags {
            let flags = flag_profile.as_ref().map(|f| f.flags.clone()).unwrap_or_default();
            let n = crate::fastflags::apply_to_install(&target.dir, &flags, &state.backups)?;
            state.logger.info("launch", format!("Applied {n} Fast Flags from {}", flag_profile.as_ref().map(|f| f.name.as_str()).unwrap_or("no profile")));
        }
        run_hooks(app, "beforeLaunch", None);
        let (pid, child) = launcher::spawn_and_watch(&launcher::player_exe(&target.dir), &args, std::time::Duration::from_secs(6)).await?;
        Ok((pid, child, target))
    }
    .await;
    let (result, child) = match result {
        Ok((pid, child, target)) => (Ok((pid, target)), child),
        Err(e) => (Err(e), None),
    };

    let record = match &result {
        Ok((pid, target)) => LaunchRecord {
            id: uuid::Uuid::new_v4().to_string(),
            at: chrono::Local::now(),
            version: target.version.clone(),
            install_path: Some(target.dir.to_string_lossy().to_string()),
            profile: profile.as_ref().map(|p| p.name.clone()),
            fast_flag_profile: flag_profile.as_ref().map(|f| f.name.clone()),
            mode: mode.into(),
            result: LaunchResult::Started,
            error: None,
            suggestions: vec![],
            pid: Some(*pid),
        },
        Err(e) => LaunchRecord {
            id: uuid::Uuid::new_v4().to_string(),
            at: chrono::Local::now(),
            version: state.install_state().active.map(|a| a.version),
            install_path: None,
            profile: profile.as_ref().map(|p| p.name.clone()),
            fast_flag_profile: flag_profile.as_ref().map(|f| f.name.clone()),
            mode: mode.into(),
            result: LaunchResult::Failed,
            error: Some(e.to_string()),
            suggestions: launcher::troubleshooting(e),
            pid: None,
        },
    };
    state.launches.push(record.clone());
    match &result {
        Ok(_) => {
            state.logger.info("launch", format!("Roblox started (pid {})", record.pid.unwrap_or(0)));
            state.activity.record(ActivityKind::Launch, "Launched Roblox", record.profile.clone().map(|p| format!("Profile: {p}")));
            if let Some(p) = &profile {
                let id = p.id.clone();
                let _ = state.update_profiles(|s| {
                    s.touch(&id);
                    Ok(())
                });
            }
            run_hooks(app, "afterLaunch", None);
            if let (Some(child), Some(pid)) = (child, record.pid) {
                start_session(app, pid, child, profile.as_ref().map(|p| p.name.clone()));
            }
            if let Some(w) = app.get_webview_window("main") {
                match prefs.after_launch {
                    AfterLaunch::Minimize => {
                        let _ = w.minimize();
                    }
                    AfterLaunch::Close => {
                        changed(app, "launch");
                        app.exit(0);
                    }
                    AfterLaunch::Nothing => {}
                }
            }
        }
        Err(e) => {
            state.logger.error("launch", format!("Launch failed: {e}"));
            state.activity.record(ActivityKind::Error, "Launch failed", Some(e.to_string()));
            if state.settings().notifications.launch_failed {
                notify(app, "error", "Roblox failed to launch", &e.to_string(), true);
            }
        }
    }
    changed(app, "launch");
    result.map(|_| record)
}

/// Track a running Roblox session: show Discord presence and record the play
/// time when the player process exits.
fn start_session(app: &AppHandle, pid: u32, mut child: std::process::Child, profile: Option<String>) {
    let state = app.state::<AppState>();
    let started_at = chrono::Local::now();
    *state.session.lock().unwrap_or_else(|p| p.into_inner()) = Some(crate::state::RobloxSession { pid, started_at, profile });
    refresh_presence(app);
    let app = app.clone();
    std::thread::Builder::new()
        .name("aeolus-roblox-session".into())
        .spawn(move || {
            let _ = child.wait();
            let state = app.state::<AppState>();
            let ended = {
                let mut s = state.session.lock().unwrap_or_else(|p| p.into_inner());
                // Only end the session if it is still this process (a newer launch may have replaced it).
                if s.as_ref().map(|x| x.pid) == Some(pid) { s.take() } else { None }
            };
            if let Some(sess) = ended {
                let mins = (chrono::Local::now() - sess.started_at).num_minutes();
                state.logger.info("launch", format!("Roblox closed after {mins} min (pid {pid})"));
                state.activity.record(ActivityKind::Launch, "Roblox closed", Some(format!("Played for {}", if mins < 1 { "under a minute".to_string() } else { format!("{mins} min") })));
                refresh_presence(&app);
                changed(&app, "activity");
            }
        })
        .ok();
}

/// Show or clear Discord Rich Presence to match the current session and settings.
pub fn refresh_presence(app: &AppHandle) {
    let state = app.state::<AppState>();
    let d = state.settings().discord;
    let session = state.session.lock().unwrap_or_else(|p| p.into_inner()).clone();
    let info = match session {
        Some(s) if d.rich_presence => Some(crate::system::discord::presence_for(s.profile.as_deref(), d.show_profile, s.started_at.timestamp())),
        _ => None,
    };
    state.presence.set(info);
}
