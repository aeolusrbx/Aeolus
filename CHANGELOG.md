# Changelog

All notable changes to Aeolus are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/) and the project uses semantic versioning.

## [0.1.2] — 2026-11-11

### Added
- Discord Rich Presence: while Roblox (started from Aeolus) is running, your Discord profile shows "Playing Aeolus · Running Roblox with Aeolus" with the Aeolus logo, elapsed time and a "Get Aeolus" button. Turn it on or off in Settings → Launch → Discord, and choose whether to show the profile name.
- Recent activity records how long you played when Roblox closes.

### Changed
- A missing release feed (404) now explains itself instead of suggesting a network problem.

## [0.1.1] — 2026-11-11

### Added
- Automatic release builds: publishing a GitHub Release builds the NSIS and MSI installers on Windows, runs all tests, and attaches the installers plus `aeolus-latest.json`.
- Self-update works out of the box: the official feed (`github.com/aeolusrbx/Aeolus`) is the default, and Aeolus checks it on startup (can be turned off in Settings → Advanced).

### Changed
- Settings schema 2: existing installs that never set a release feed are migrated to the official one; a custom feed is kept.

## [0.1.0] — 2026-11-11

### Added
- First release of Aeolus: a Tauri 2 desktop app with a Rust backend and a React/TypeScript frontend.
- Official Aeolus wind logo used for the window, sidebar, loading screen, About page, executable, installer and tray icon.
- Installation management: detection of Aeolus, official, Bloxstrap and Fishstrap installs; install, update, quick/full verify, repair, reinstall and uninstall; interrupted-install recovery; configurable install folder; download cache.
- Roblox update checks with history and notifications; optional update before launch.
- Launch profiles with icon, accent, Fast Flag profile, auto-update, verify and after-launch behavior; launch history; troubleshooting suggestions.
- Fast Flags editor: profiles (create, duplicate, rename, delete, default, compare, import, export, reset, restore), validation, allowlist status, categories, search, filter, sort, apply now.
- Customization: themes, accent color, sidebar style, compact navigation, dashboard background, logo presentation, animation level.
- Declarative plugin system (API v1) with permissions, approval, integrity pinning, dependencies, configuration and lifecycle hooks; example "Graphics Presets" plugin.
- Diagnostics: system information, log viewer (search, severity and date filters, export, clear), redacted diagnostic reports.
- Windows integration: custom title bar, remembered window state, tray menu, launch at startup, native notifications, single instance, optional `roblox-player:` link handling, checksum-verified self-updater.
- Configuration: atomic JSON storage, schema validation, versioned migrations, automatic backups, corruption recovery, full import/export and data deletion.
- Tests: 31 Rust unit tests, 4 Rust integration tests (installer pipeline against a mock CDN), 16 Vitest UI tests.
