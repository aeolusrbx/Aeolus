<p align="center"><img src="assets/brand/aeolus-icon-1024.png" width="120" alt="Aeolus"></p>

<h1 align="center">Aeolus</h1>
<p align="center">A Roblox bootstrapper and client manager for Windows.</p>

<p align="center"><a href="https://aeolusrbx.github.io/Aeolus/"><b>aeolusrbx.github.io/Aeolus</b></a></p>

<p align="center">
  <a href="https://discord.gg/Bk7HBAmCh7"><img src="https://img.shields.io/badge/Discord-Join%20the%20server-5865F2?logo=discord&logoColor=white" alt="Join the Aeolus Discord"></a>
  <a href="https://github.com/aeolusrbx/Aeolus/releases/latest"><img src="https://img.shields.io/github/v/release/aeolusrbx/Aeolus?label=Download&color=FF861F" alt="Latest release"></a>
  <a href="https://github.com/aeolusrbx/Aeolus/releases"><img src="https://img.shields.io/github/downloads/aeolusrbx/Aeolus/total?color=292C34" alt="Downloads"></a>
</p>

Aeolus installs, updates and launches the Roblox Player using Roblox's **official** distribution, and gives you a clean place to manage Fast Flags, launch profiles, plugins and diagnostics.

Aeolus is a bootstrapper, not an executor. It never injects code, never touches Roblox's anti-cheat, and never asks for or stores your Roblox password or account cookies.

> Aeolus is an independent project and is not affiliated with or endorsed by Roblox Corporation.

## Features

| Area | What it does |
|---|---|
| **Installations** | Detects Aeolus, official, Bloxstrap and Fishstrap installs. Installs Roblox from `setup.rbxcdn.com` with MD5 verification of every package, resumable downloads, staged extraction and an atomic swap. Includes quick/full verify, repair, reinstall and uninstall. |
| **Updates** | Checks `clientsettingscdn.roblox.com` for the current version, keeps an update history and can update before each launch. Failed updates leave the working installation in place. |
| **Launch** | Launch profiles (icon, accent, Fast Flag profile, auto-update, verify, after-launch behavior), launch history and troubleshooting for failed launches. Optionally handles `roblox-player:` links from the website. |
| **Games** | Detects the game and server you join from Roblox's log: game history, server location, Rejoin the same server, your game in Discord Rich Presence. After a disconnect it offers to rejoin (or rejoins automatically for connection errors), and when Roblox crashes it explains the likely cause and what to try. |
| **FPS cap** | Sets Roblox's own frame rate limit (presets or a custom number) before each launch, globally or per profile. |
| **Fast Flags** | A searchable, sortable, filterable editor with multiple profiles, type validation, duplicate detection, allowlist status, import/export, compare, apply-now, and automatic backups with restore. |
| **Customization** | Dark, light and system themes; accent color; sidebar style; compact navigation; dashboard background; logo presentation; animation level. |
| **Plugins** | Declarative plugins with explicit permissions, user approval, integrity pinning (a changed plugin is disabled until re-approved), dependencies, configuration and lifecycle hooks. Includes an example plugin. |
| **Diagnostics** | System information, a structured log viewer (search, severity and date filters, export, clear) and redacted diagnostic reports. |
| **Windows** | Custom title bar, remembered window size and position, system tray, launch at startup, native notifications, single instance, and a checksum-verified self-updater. |

## Community

Questions, bug reports and mod sharing: **[join the Aeolus Discord](https://discord.gg/Bk7HBAmCh7)**.

### About Fast Flags

Since 30 September 2025, Roblox only applies Fast Flags that are on its [published allowlist](https://devforum.roblox.com/t/allowlist-for-local-client-configuration-via-fast-flags/3966569) and silently ignores the rest. Aeolus still lets you edit any flag, but it labels each one as **Allowlisted**, **Not allowlisted** or **Invalid**, so a flag is never presented as working just because it is valid JSON.

## Installing

1. Run `Aeolus_0.1.0_x64-setup.exe`. It installs per user, so it doesn't need administrator rights.
2. Windows SmartScreen may warn you because the installer is not code-signed yet. Choose **More info → Run anyway** if you trust the build.
3. Start Aeolus from the Start menu. Open **Installations → Install Roblox**.

To uninstall, use **Settings → Apps → Installed apps → Aeolus → Uninstall**. Your configuration stays in `%APPDATA%\dev.aeolus.app` unless you choose to delete app data. Upgrading with a newer installer keeps your configuration.

## Where Aeolus keeps data

| Path | Contents |
|---|---|
| `%APPDATA%\dev.aeolus.app` | settings, profiles, Fast Flags, plugins, backups |
| `%LOCALAPPDATA%\dev.aeolus.app\Roblox\Versions` | the Aeolus-managed Roblox Player (configurable) |
| `%LOCALAPPDATA%\dev.aeolus.app\downloads` | verified package cache (resumable) |
| `%LOCALAPPDATA%\dev.aeolus.app\logs` | JSON-lines logs, one file per day |

## Development

See [documentation/BUILDING.md](documentation/BUILDING.md) for how to build, [documentation/ARCHITECTURE.md](documentation/ARCHITECTURE.md) for how the code fits together, and [documentation/PLUGINS.md](documentation/PLUGINS.md) for how to write a plugin.

```powershell
npm install
npm run tauri dev       # run with hot reload
npm test                # frontend tests (Vitest)
cd src-tauri; cargo test  # backend unit + integration tests
npm run tauri build     # release build: NSIS + MSI installers
```

Project status lives in [FEATURE_STATUS.md](FEATURE_STATUS.md), plans in [ROADMAP.md](ROADMAP.md), and changes in [CHANGELOG.md](CHANGELOG.md).
