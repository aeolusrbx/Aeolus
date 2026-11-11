//! Aeolus - Roblox bootstrapper and client manager.

pub mod commands;
pub mod configuration;
pub mod diagnostics;
pub mod error;
pub mod fastflags;
pub mod installation;
pub mod launcher;
pub mod paths;
pub mod plugins;
pub mod profiles;
pub mod services;
pub mod skies;
pub mod state;
pub mod system;
pub mod tray;
pub mod updates;

use std::sync::atomic::Ordering;
use tauri::Manager;

fn find_link(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find(|a| {
            let l = a.to_ascii_lowercase();
            l.starts_with("roblox-player:") || l.starts_with("roblox:")
        })
        .cloned()
}

fn handle_link(app: &tauri::AppHandle, link: String, exit_after: bool) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        match services::launch(&app, Some(link), None).await {
            Ok(_) if exit_after => app.exit(0),
            Ok(_) => {}
            Err(_) => {
                // Make sure the user sees what went wrong.
                app.state::<state::AppState>().started_minimized.store(false, Ordering::SeqCst);
                tray::show_main(&app);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(link) = find_link(&argv) {
                handle_link(app, link, false);
            } else {
                tray::show_main(app);
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--minimized"])))
        .setup(|app| {
            let handle = app.handle().clone();
            let config = app.path().app_config_dir()?;
            let local = app.path().app_local_data_dir()?;
            let state = state::AppState::init(paths::Paths::new(config, local)).map_err(|e| {
                eprintln!("Aeolus failed to initialize: {e}");
                Box::<dyn std::error::Error>::from(e.to_string())
            })?;
            let settings = state.settings();
            state.logger.info("app", format!("Aeolus {} starting on {}", env!("CARGO_PKG_VERSION"), std::env::consts::OS));
            state.logger.enforce_retention(settings.privacy.log_retention_days);

            let args: Vec<String> = std::env::args().collect();
            let link = find_link(&args);
            let minimized = link.is_some() || args.iter().any(|a| a == "--minimized") || settings.general.start_minimized;
            state.started_minimized.store(minimized, Ordering::SeqCst);
            app.manage(state);

            if let Err(e) = tray::create(&handle) {
                app.state::<state::AppState>().logger.error("tray", format!("Could not create tray icon: {e}"));
            }

            if let Some(link) = link {
                handle_link(&handle, link, true);
            } else {
                if settings.installation.check_updates_on_startup {
                    let h = handle.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = services::check_updates(&h, true).await;
                    });
                }
                if settings.advanced.check_aeolus_updates_on_startup && !settings.advanced.self_update_feed.trim().is_empty() {
                    let h = handle.clone();
                    let feed = settings.advanced.self_update_feed.clone();
                    tauri::async_runtime::spawn(async move {
                        let st = h.state::<state::AppState>();
                        match system::selfupdate::check(&st.http, &feed, env!("CARGO_PKG_VERSION")).await {
                            Ok(c) if c.update_available => {
                                let v = c.latest.map(|l| l.version).unwrap_or_default();
                                st.logger.info("self-update", format!("Aeolus {v} is available"));
                                services::notify(&h, "info", "Aeolus update available", &format!("Version {v} is ready. Install it from Settings → Advanced."), true);
                            }
                            Ok(_) => st.logger.debug("self-update", "Aeolus is up to date"),
                            Err(e) => st.logger.warn("self-update", format!("Aeolus update check failed: {e}")),
                        }
                    });
                }
            }

            // Fallback: if the UI never reports ready (e.g. a frontend error), still show the window.
            let h = handle.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                let st = h.state::<state::AppState>();
                if !st.started_minimized.load(Ordering::SeqCst) {
                    if let Some(w) = h.get_webview_window("main") {
                        if !w.is_visible().unwrap_or(true) {
                            let _ = w.show();
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::app_ready,
            commands::app::get_settings,
            commands::app::save_settings,
            commands::app::reset_settings,
            commands::app::system_integration,
            commands::app::activity_list,
            commands::app::activity_clear,
            commands::app::self_update_check,
            commands::app::self_update_install,
            commands::app::cancel_operation,
            commands::app::quit_app,
            commands::app::close_window,
            commands::app::export_all_data,
            commands::app::import_all_data,
            commands::app::delete_user_data,
            commands::app::list_backups,
            commands::install::install_overview,
            commands::install::check_for_updates,
            commands::install::run_install_operation,
            commands::install::verify_installation,
            commands::install::uninstall_roblox,
            commands::install::discard_incomplete,
            commands::install::clear_download_cache,
            commands::install::set_launch_install,
            commands::install::set_install_root,
            commands::install::update_history,
            commands::install::launch_roblox,
            commands::install::launch_history,
            commands::install::clear_launch_history,
            commands::config::ff_profiles,
            commands::config::ff_rows,
            commands::config::ff_create,
            commands::config::ff_duplicate,
            commands::config::ff_rename,
            commands::config::ff_delete,
            commands::config::ff_set_flag,
            commands::config::ff_remove_flags,
            commands::config::ff_reset,
            commands::config::ff_import,
            commands::config::ff_import_file,
            commands::config::ff_export,
            commands::config::ff_export_file,
            commands::config::ff_compare,
            commands::config::ff_set_default,
            commands::config::ff_apply_now,
            commands::config::ff_plugin_presets,
            commands::config::ff_allowlist,
            commands::config::profiles_get,
            commands::config::profile_create,
            commands::config::profile_update,
            commands::config::profile_duplicate,
            commands::config::profile_delete,
            commands::config::profile_set_default,
            commands::config::profile_set_active,
            commands::config::profile_export_file,
            commands::config::profile_import_file,
            commands::config::backups_overview,
            commands::config::restore_backup,
            commands::config::create_manual_backup,
            commands::extras::diagnostics_system,
            commands::extras::logs_query,
            commands::extras::logs_clear,
            commands::extras::logs_export,
            commands::extras::diagnostics_report,
            commands::extras::plugins_list,
            commands::extras::plugin_enable,
            commands::extras::plugin_disable,
            commands::extras::plugin_set_config,
            commands::extras::plugin_install,
            commands::extras::plugin_uninstall,
            commands::skies::skies_overview,
            commands::skies::sky_import,
            commands::skies::sky_rename,
            commands::skies::sky_delete,
            commands::skies::sky_set_global,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Aeolus")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(st) = app.try_state::<state::AppState>() {
                    st.cancel_operation();
                    st.presence.shutdown();
                    st.logger.info("app", "Aeolus exiting");
                }
            }
        });
}
