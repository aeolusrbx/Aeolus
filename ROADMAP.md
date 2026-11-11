# Roadmap

## 0.1 — Foundation (current)
- [x] Project, branding, icons from the official wind logo
- [x] Custom window, sidebar, themes, notifications, dialogs
- [x] Installation detection, install, update, verify, repair, reinstall, uninstall
- [x] Resumable, MD5-verified downloads; staged install with atomic swap
- [x] Launch profiles, launch history, troubleshooting
- [x] Fast Flags editor with profiles, allowlist status, backups
- [x] Declarative plugin system (API v1) with permissions and integrity pinning
- [x] Diagnostics, structured logs, redacted reports
- [x] Tray, startup registration, roblox-player link handling, self-updater
- [x] NSIS installer (cross-compiled)

## 0.2 — Release engineering
- [ ] Code-sign the executable and installer (removes the SmartScreen warning)
- [ ] Host releases and publish a self-update feed (`aeolus-latest.json`)
- [ ] MSI build on a Windows CI runner (GitHub Actions `windows-latest`)
- [ ] End-to-end tests driving the real app (tauri-driver / WebDriver)

## 0.3 — Quality of life
- [ ] Fetch the current Fast Flag allowlist instead of the built-in baseline, if Roblox publishes it in a machine-readable form
- [ ] Optional deployment channel selection (only channels Roblox allows publicly)
- [ ] Localization framework (English is the only language today)
- [ ] Move the managed installation when the install folder changes (today the next install goes to the new folder)

## Later
- [ ] Plugin marketplace: search, categories, compatibility, install and update. The manifest already carries the needed metadata.
- [ ] Plugin API v2 (more host actions, still without running arbitrary code)
