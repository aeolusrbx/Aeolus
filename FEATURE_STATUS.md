# Feature status

Status levels:

- **Tested**: covered by automated tests that pass, or checked by hand where noted.
- **Implemented**: code complete and wired to the UI, but not yet exercised end to end.
- **In progress**: partly done.
- **Blocked**: cannot be completed without something outside the codebase.
- **Not started**: no work yet.

Evidence key:

- **R**: Rust unit or integration test (`cargo test`).
- **V**: Vitest UI test against the mock backend.
- **W**: verified on a real Windows 11 PC (see "Windows verification" below).

_Last updated: 2026-11-11 (v0.1.0)_

## Core

| Feature | Status | Evidence / notes |
|---|---|---|
| Project structure, Tauri 2 + React + TS + Vite + Tailwind | Tested | builds; `tsc` strict 0 errors |
| Branding: official logo on window, sidebar, loading, About, exe, installer, tray | Tested (W) | icons generated from supplied PNG without redesign |
| Custom title bar (move, minimize, maximize, close) | Implemented | renders on Windows (W); buttons not clicked during verification |
| Window size/position persistence | Implemented | tauri-plugin-window-state |
| Sidebar: expand/collapse, remembered, active indicator, version | Tested (V) | |
| Notifications (toasts): 4 levels, auto-dismiss, manual dismiss | Tested (V) | |
| Dialogs: confirm/delete/reset/install/import/export/error | Tested (V) | confirm + cancel paths |

## Installation & updates

| Feature | Status | Evidence / notes |
|---|---|---|
| Detect installs (Aeolus, official, Program Files, Bloxstrap, Fishstrap) | Tested (R) | uses env-var paths, nothing hard-coded |
| Detect incomplete installs / resume | Tested (R) | staging dirs reported; Range resume |
| Fetch version from clientsettingscdn | Tested (R, W) | |
| Parse rbxPkgManifest, MD5-verify packages | Tested (R) | mismatch → integrity error, nothing installed |
| Resumable downloads (HTTP Range) | Tested (R) | interrupted transfer resumes |
| Staged extraction, zip-slip protection, atomic swap with rollback | Tested (R) | |
| Install / Update / Repair / Reinstall | Tested (R, W: install) | live install from Roblox CDN verified on Windows |
| Verify (quick size check / full SHA-256) | Tested (R, W) | detects missing and modified files |
| Uninstall (refuses non-Aeolus folders), backup first | Tested (R) | |
| Cancellation | Tested (R) | |
| Progress: bytes, speed, ETA, stage | Tested (R: events) | speed/ETA only shown when measurable |
| Permission/invalid directory handling | Tested (R) | write probe before downloading |
| Update history | Implemented | |
| Custom install directory | Implemented | existing install isn't moved (see ROADMAP) |

## Launch & profiles

| Feature | Status | Evidence / notes |
|---|---|---|
| Launch (`--app`), link launch (`roblox-player:`/`roblox:`) | Tested (R, W: app launch) | link launch not yet tried on Windows |
| Profiles: create/edit/duplicate/rename/delete/default/active/export/import | Tested (R, V: active switch) | |
| Launch history, failure recording, troubleshooting | Tested (V: history), Implemented (UI) | |
| Update/verify/apply-flags before launch | Implemented | |
| After-launch: minimize / close | Implemented | |

## Fast Flags

| Feature | Status | Evidence / notes |
|---|---|---|
| Table: search, filter (category, status), sort | Tested (V) | |
| Add / edit / delete / reset | Tested (R, V: add) | |
| Type validation, invalid names, duplicates (case-insensitive) | Tested (R) | |
| Allowlist status (baseline list from Roblox, 29 Sep 2025) | Tested (R, V) | Roblox may change the list without notice |
| Profiles: duplicate/rename/export/import/compare/restore | Tested (R) | |
| Backups before every change + restore | Tested (R) | |
| Apply to ClientSettings/ClientAppSettings.json (with backup) | Tested (R) | |
| Deprecated-flag warnings | Not started | no verifiable source for deprecations |

## Customization

| Feature | Status | Evidence / notes |
|---|---|---|
| Dark / Light / System theme | Tested (V) | |
| Accent presets, color picker, hex input | Tested (V) | |
| Sidebar style, compact nav, dashboard background, logo presentation | Implemented | |
| Animations: enabled / reduced / disabled | Implemented | |

## Plugins

| Feature | Status | Evidence / notes |
|---|---|---|
| Manifest v1, validation, permission enforcement | Tested (R) | |
| Approval, integrity pinning (changed → disabled) | Tested (R) | |
| Dependencies (installed, min version, enabled) | Tested (R) | |
| Config API, lifecycle hooks (log/notify) | Tested (R: render/config), Implemented (hooks) | |
| Install from folder, uninstall | Tested (R) | |
| Example plugin | Tested (R) | seeded disabled on first run |
| Marketplace | Not started | described in UI and docs; no fake listing |

## Diagnostics & logging

| Feature | Status | Evidence / notes |
|---|---|---|
| System info (OS, build, CPU, GPU, RAM, arch, display) | Tested (W) | |
| Logs: write, search, level/date filter, export, clear, retention | Tested (R, V: level filter) | |
| Secret redaction (cookies, tokens, user name in paths) | Tested (R) | |
| Diagnostic report export | Implemented | |

## Windows integration

| Feature | Status | Evidence / notes |
|---|---|---|
| Tray (open, launch, check updates, exit) | Implemented | |
| Launch at startup (HKCU Run, reversible) | Implemented | tauri-plugin-autostart |
| Start minimized | Implemented | |
| Native notifications with preferences | Implemented | |
| Single instance (forwards links) | Implemented | |
| Custom skies (.tex packs, global + per-profile, restore originals) | Tested (R, V) | not yet tried with a real sky pack in Roblox |
| Discord Rich Presence (toggle, profile name, play time) | Tested (R, W) | seen in Discord on 2026-11-11; another app's presence (e.g. Medal) can take priority |
| roblox-player link handler with restore of previous handler | Implemented | |
| Self-updater (feed, SHA-256 verify, run installer after exit) | Tested (W) | 0.1.0 → 0.1.1 and 0.1.1 → 0.1.2 via Check now, from GitHub Releases on 2026-11-11 |

## Configuration

| Feature | Status | Evidence / notes |
|---|---|---|
| Atomic JSON, schema validation, defaults | Tested (R) | |
| Versioned migrations; newer files refused | Tested (R) | |
| Corruption recovery (file preserved) | Tested (R) | |
| Import / export / delete all data | Implemented | |

## Release

| Feature | Status | Evidence / notes |
|---|---|---|
| NSIS installer (per-user, shortcuts, uninstaller, upgrades keep config) | Tested (W: install) | uninstall/upgrade not yet run |
| MSI installer | Implemented | built by the GitHub release workflow on Windows; not yet installed by hand |
| Code signing | Not started | SmartScreen will warn until signed |

## Windows verification

Done on 2026-11-11 on Windows 11 Home (build 26200), with an i7-13700F, an RTX 4070 and a 2560×1440 display, using the cross-compiled `Aeolus_0.1.0_x64-setup.exe`:

| Check | Result |
|---|---|
| NSIS installer: welcome → location (`%LOCALAPPDATA%\Aeolus`) → install → finish, desktop shortcut option | Passed |
| App starts, custom title bar and sidebar render, logo shown | Passed |
| Startup update check against live `clientsettingscdn.roblox.com` | Passed (found 0.747.0.7471481) |
| Install Roblox from the live Roblox CDN (232 MB, MD5-verified) | Passed: 2,857 files in 8.2 s |
| Full SHA-256 verify | Passed: 2,857 checked, 0 missing, 0 damaged |
| Detection of the official Roblox install alongside the Aeolus one | Passed |
| Launch Roblox from Aeolus (`RobloxPlayerBeta.exe --app`) | Passed: process started, launch recorded |
| Diagnostics: OS, build, CPU, GPU (via CIM), RAM, display, live logs | Passed |

Not yet exercised on Windows: tray menu, launch at startup, `roblox-player:` link handling, notifications, uninstalling Aeolus, and upgrading over an older version. These features are **Implemented** but not yet verified on Windows.

## Outstanding bugs / limitations

- The installer and exe are unsigned, so Windows SmartScreen shows a warning.
- The Fast Flag allowlist is the published baseline; Aeolus can't fetch the live list.
- Changing the install folder doesn't move an existing installation.
