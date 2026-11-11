# Architecture

Aeolus is a Tauri 2 app: a Rust backend that does all file, network and process work, and a React + TypeScript frontend that only calls typed commands.

```
src/                      React frontend
  services/api.ts         one typed wrapper per Rust command
  stores/                 app (settings, navigation, progress), toasts, dialogs
  hooks/                  useResource (fetch + refresh on backend events), useRobloxActions
  components/             title bar, sidebar, progress dock, dialogs, UI primitives
  pages/                  Home, Installations, Launch, FastFlags, Customization, Plugins, Diagnostics, Settings
src-tauri/src/
  lib.rs                  app setup: plugins, tray, single instance, link handling, command registry
  state.rs                AppState: all persisted documents plus the single "current operation" slot
  services.rs             install/update, launch and plugin-hook flows (shared by commands and tray)
  commands/               thin #[tauri::command] layer
  configuration/          atomic JSON store, backups, history lists, settings schema, migrations
  installation/           manifest parsing, download/verify/extract, detection, progress
  updates/                update status and history records
  launcher/               argument validation, spawn + early-exit detection, troubleshooting
  fastflags/              flag profiles, validation, allowlist, apply to ClientAppSettings.json
  profiles/               unified launch profiles
  plugins/                declarative plugin host (API v1)
  diagnostics/            system info, structured logging with redaction, activity feed
  system/                 roblox-player: protocol registration, self-updater
  tray/                   system tray
```

## Events

The backend emits:

- `aeolus://progress`: progress for the running operation (real byte counts, speed, ETA).
- `aeolus://changed`: an area changed (`install`, `fastflags`, `profiles`, `settings`, `plugins`, `launch`, `activity`, `all`). `useResource` refetches when its area changes.
- `aeolus://toast`: messages raised by background work, such as tray actions.
- `aeolus://navigate`: page requests from the tray.

## Persistence

Every document is written atomically (temp file, `fsync`, then rename). Corrupt files are never deleted: they are renamed to `*.corrupt-<timestamp>.json` or copied to `backups/`, and defaults are used instead. Settings carry `schemaVersion`; `configuration/migrations.rs` upgrades older files step by step and refuses files written by a newer Aeolus rather than overwriting them.

## Installation pipeline

1. `GET clientsettingscdn.roblox.com/v2/client-version/WindowsPlayer` returns the version and deployment id.
2. `GET setup.rbxcdn.com/<id>-rbxPkgManifest.txt` returns the package list with MD5 and sizes. Mirrors are tried in order.
3. Each package is downloaded into `downloads/<md5>.part`. Interrupted transfers resume with HTTP Range, and a finished package is MD5-verified before being renamed to `<md5>.zip`.
4. Packages are extracted into `Versions/<id>.aeolus-staging` using a package-to-folder map, with path-traversal protection. A SHA-256 of every file is recorded in `aeolus-files.json`.
5. The staging folder is renamed into place, replacing any previous copy with rollback. Older Aeolus-managed versions are removed. Folders without an Aeolus index are never deleted.

Only one long operation runs at a time (`AppState::begin_operation`), and every operation can be cancelled.
