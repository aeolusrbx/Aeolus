//! System tray icon and menu.

use crate::services;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Aeolus", true, None::<&str>)?;
    let launch = MenuItem::with_id(app, "launch", "Launch Roblox", true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", "Check for updates", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "Exit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &launch, &updates, &sep, &quit])?;

    let icon = tauri::image::Image::from_bytes(include_bytes!("../../icons/tray.png"))?;
    TrayIconBuilder::with_id("aeolus-tray")
        .icon(icon)
        .tooltip("Aeolus")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "launch" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = services::launch(&app, None, None).await;
                });
            }
            "updates" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    match services::check_updates(&app, true).await {
                        Ok(s) if !s.update_available => services::notify(&app, "success", "Roblox is up to date", s.installed.as_deref().unwrap_or("Not installed"), true),
                        Ok(_) => {}
                        Err(e) => services::notify(&app, "error", "Update check failed", &e.to_string(), true),
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
                let _ = tray.app_handle().emit(services::EV_NAVIGATE, "home");
            }
        })
        .build(app)?;
    Ok(())
}
