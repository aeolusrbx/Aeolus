# Changelog

All notable changes to Aeolus are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/) and the project uses semantic versioning.

## [0.2.0] — 2026-11-12

### Added
- **FPS cap**: choose 30, 60, 120, 144, 165 or 240 FPS, or type any number up to 9999. Set it in Settings → Launch, or per launch profile. Aeolus writes it to Roblox's own setting before launch. Roblox officially supports up to 240, so higher values may be ignored.
- **Rejoin after a disconnect**: when you lose connection to a game, Aeolus shows a "Disconnected" card on Home and a notification with a Rejoin button that brings you back to the same server. You can also choose "Rejoin automatically", which waits 10 seconds so you can cancel. Automatic rejoin only happens for connection errors (never after a kick), and at most 3 times in 10 minutes.
- **Crash help**: when Roblox crashes or closes with an error, Home explains the likely cause in plain English (graphics driver, memory, overlays and more) and lists what to try.
- **Discord**: a "Join our Discord" button in Settings → About and in the sidebar.

## [0.1.6] — 2026-11-11

### Added
- **Game detection**: Aeolus reads Roblox's log while it runs to see which game and server you joined.
- **Discord shows the game** you're playing: its name, icon and creator, server location, and a "View game" button (toggle in Settings → Launch).
- **Server location**: city, region and country of the Roblox server, with an optional notification when you join.
- **Game history** on the Launch page (and recent games on Home) with play time and location.
- **Rejoin** the exact same server, or **Play** the game in any server. Private and reserved servers can only use Play.

## [0.1.5] — 2026-11-11

### Added
- **Mods page** (new in the sidebar): replace Roblox's built-in sounds (death, footsteps, jump, landing, falling, getting up, swimming, explosion, water splash), cursors (normal and far) and fonts (interface font, emoji font) with your own files.
- Presets: **Classic oof** death sound (uses the copy Roblox still ships) and Dot / Crosshair / Ring cursors.
- Each mod can be switched on and off or reset to Roblox's original; launch profiles have a "Use mods" switch.
- Custom skies moved to the Mods page.

### Changed
- Sky and mods share one system that saves Roblox's originals and restores them exactly.

## [0.1.4] — 2026-11-11

### Fixed
- Custom skies failed with "This Roblox version has no sky files matching…". Current Roblox keeps the skybox textures in `PlatformContent\pc\textures\sky`, not `content\sky`; Aeolus now writes them there.

## [0.1.3] — 2026-11-11

### Added
- Custom skies: add sky packs (a folder or .zip with Roblox's six `sky512_*.tex` textures, plus an optional preview image) in Customization → Sky, and pick one for all launches. Launch profiles can choose their own sky or force Roblox's default.
- Aeolus saves Roblox's original sky files before replacing them and restores them when you switch back, delete the pack, or pick "Roblox default". Only files Roblox already has are replaced.
- Verify ignores files replaced by a custom sky, so it doesn't report them as damaged.

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
