// In-memory stand-in for the Rust backend, used by UI tests and for previewing the
// interface in a browser (VITE_MOCK=1). It is never included in release builds.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";

type Any = Record<string, any>;
const now = () => new Date().toISOString();
const ago = (m: number) => new Date(Date.now() - m * 60000).toISOString();

const ALLOW = ["DFIntTextureQualityOverride", "DFFlagTextureQualityOverrideEnabled", "FIntDebugForceMSAASamples", "FFlagDebugSkyGray", "FIntFRMMaxGrassDistance", "FIntFRMMinGrassDistance", "FFlagDebugGraphicsPreferD3D11"];

function typeOf(n: string) {
  if (/^(DF|SF|F)Flag./.test(n)) return "boolean";
  if (/^(DF|SF|F)Int./.test(n)) return "integer";
  if (/^(DF|SF|F)String./.test(n)) return "string";
  if (/^(DF|F)Log./.test(n)) return "log";
  return "unknown";
}

export function createState() {
  const settings: Any = {
    schemaVersion: 1,
    general: { theme: "dark", accent: "#FF861F", language: "en", launchAtStartup: false, startMinimized: false, minimizeToTray: false, defaultPage: "home", sidebarCollapsed: false },
    appearance: { animations: "enabled", compactNavigation: false, dashboardBackground: "glow", logoPresentation: "full", sidebarStyle: "solid" },
    installation: { customInstallRoot: null, scanOfficialInstall: true, scanOtherBootstrappers: true, backupBeforeDestructive: true, keepBackups: 10, checkUpdatesOnStartup: true, updateBeforeLaunch: true, keepDownloadCache: false },
    launch: { defaultProfileId: "p1" },
    fastFlags: { defaultProfileId: null, backupOnChange: true, maxBackups: 25, warnNotAllowlisted: true, blockInvalidValues: true },
    plugins: { requireReapprovalOnChange: true, notifyOnErrors: true },
    notifications: { native: true, updateAvailable: true, updateCompleted: true, updateFailed: true, versionChanged: true, launchFailed: true, pluginEvents: false },
    discord: { richPresence: true, showProfile: true },
    privacy: { logRetentionDays: 14, includeLogsInReports: true },
    advanced: { developerMode: false, debugLogging: false, handleRobloxLinks: false, selfUpdateFeed: "", checkAeolusUpdatesOnStartup: true },
  };
  const flags: Any[] = [
    { id: "f1", name: "Default", description: "No overrides", flags: {}, modifiedAt: ago(500) },
    { id: "f2", name: "Graphics", description: "", flags: { DFFlagTextureQualityOverrideEnabled: true, DFIntTextureQualityOverride: 3, FIntDebugForceMSAASamples: 4 }, modifiedAt: ago(90) },
    { id: "f3", name: "Performance", description: "", flags: { DFFlagTextureQualityOverrideEnabled: true, DFIntTextureQualityOverride: 0, FIntFRMMaxGrassDistance: 0, DFIntTaskSchedulerTargetFps: 240, FFlagBadValue: "maybe" }, modifiedAt: ago(30) },
    { id: "f4", name: "Custom", description: "", flags: {}, modifiedAt: ago(10) },
  ];
  const profiles: Any = {
    schemaVersion: 1,
    defaultId: "p1",
    activeId: "p1",
    profiles: [
      { id: "p1", name: "Default", description: "Standard launch", icon: "wind", accent: "#FF861F", fastFlagProfileId: null, launch: { applyFastFlags: true, updateBeforeLaunch: true, verifyBeforeLaunch: false, afterLaunch: "nothing" }, createdAt: ago(5000), lastUsedAt: ago(42) },
      { id: "p2", name: "Competitive", description: "Low latency, low detail", icon: "zap", accent: "#3E8EF7", fastFlagProfileId: "f3", launch: { applyFastFlags: true, updateBeforeLaunch: true, verifyBeforeLaunch: true, afterLaunch: "minimize" }, createdAt: ago(3000), lastUsedAt: null },
    ],
  };
  const install: Any = {
    schemaVersion: 1,
    active: { version: "0.701.0.7010921", versionGuid: "version-8f3c21d9a4be4c10", path: "C:\\Users\\<user>\\AppData\\Local\\dev.aeolus.app\\Roblox\\Versions\\version-8f3c21d9a4be4c10", installedAt: ago(1440), fileCount: 2841, size: 412_000_000 },
    preferredExternal: null,
    lastCheck: { checkedAt: ago(3), latest: { version: "0.702.0.7020455", versionGuid: "version-2d7e90aa1f6c4b33" }, error: null },
  };
  const activity: Any[] = [
    { id: "a1", kind: "launch", title: "Launched Roblox", detail: "Profile: Default", at: ago(42) },
    { id: "a2", kind: "config", title: "Updated Fast Flag profile 'Performance'", detail: null, at: ago(55) },
    { id: "a3", kind: "update", title: "Updated to 0.701.0.7010921", detail: "2841 files", at: ago(1440) },
    { id: "a4", kind: "backup", title: "Configuration backed up before reinstall", detail: "4 files", at: ago(1500) },
    { id: "a5", kind: "error", title: "Update check failed", detail: "Network error: connection timed out", at: ago(2900) },
  ];
  const logs: Any[] = [
    { ts: ago(1), level: "INFO", source: "updates", message: "Latest Roblox version is 0.702.0.7020455 (version-2d7e90aa1f6c4b33)" },
    { ts: ago(42), level: "INFO", source: "launch", message: "Roblox started (pid 18244)" },
    { ts: ago(42), level: "INFO", source: "launch", message: "Applied 0 Fast Flags from no profile" },
    { ts: ago(2900), level: "WARNING", source: "updates", message: "Update check failed: connection timed out" },
    { ts: ago(3000), level: "ERROR", source: "install", message: "Updating Roblox failed: Network error: connection reset" },
  ];
  const plugins: Any[] = [
    {
      id: "dev.aeolus.example.graphics-presets",
      status: "needs-approval",
      error: null,
      permissions: [["log", "Write entries to the Aeolus log"], ["notifications", "Show notifications"], ["fastflags.presets", "Offer Fast Flag presets you can import into a profile"], ["launch.hooks", "Run its hooks before and after Roblox launches"]],
      dependenciesOk: true,
      dependencyIssues: [],
      config: { launchMessage: "Have a good session!" },
      path: "C:\\Users\\<user>\\AppData\\Roaming\\dev.aeolus.app\\plugins\\dev.aeolus.example.graphics-presets",
      lastUpdated: ago(600),
      manifest: {
        id: "dev.aeolus.example.graphics-presets", name: "Graphics Presets", version: "1.0.0", author: "Aeolus", aeolusApi: 1,
        description: "Example plugin: contributes Fast Flag presets built only from Roblox-allowlisted flags, and shows a notification when Roblox launches.",
        permissions: ["log", "notifications", "fastflags.presets", "launch.hooks"], dependencies: [],
        contributes: { fastFlagPresets: [{ name: "Crisp textures", description: "Forces the highest texture quality.", flags: { DFIntTextureQualityOverride: 3 } }], configuration: [{ key: "launchMessage", label: "Launch message", type: "string", default: "Have a good session!", description: "Shown when Roblox starts." }], hooks: { afterLaunch: [{}] } },
      },
    },
  ];
  return { settings, flags, profiles, install, activity, logs, plugins, launches: [] as Any[], calls: [] as { cmd: string; args: Any }[] };
}

export type MockState = ReturnType<typeof createState>;

function rows(p: Any) {
  const names = Object.keys(p.flags);
  return names.map((name) => {
    const value = p.flags[name];
    const t = typeOf(name);
    const okType = t === "boolean" ? typeof value === "boolean" : t === "integer" || t === "log" ? Number.isInteger(value) : t === "string" ? typeof value === "string" : false;
    const status = !okType ? "invalid" : ALLOW.includes(name) ? "allowlisted" : "notAllowlisted";
    const category = /Texture|Grass|FRM/.test(name) ? "graphics" : /MSAA|D3D|Sky/.test(name) ? "rendering" : /Fps|TaskScheduler/.test(name) ? "performance" : "other";
    return { name, value, flagType: t, category, status, messages: status === "invalid" ? ["Wrong type"] : status === "notAllowlisted" ? ["Not on Roblox's known allowlist"] : [] };
  });
}

export function handler(s: MockState) {
  const err = (message: string, kind = "validation") => Promise.reject({ kind, message, hint: null });
  const ff = (id: string) => s.flags.find((f) => f.id === id) as Any;
  return async (cmd: string, args: Any = {}): Promise<unknown> => {
    s.calls.push({ cmd, args });
    if (cmd.startsWith("plugin:event|")) return 1;
    if (cmd.startsWith("plugin:window|")) return cmd.endsWith("is_maximized") ? false : null;
    switch (cmd) {
      case "app_info":
        return { name: "Aeolus", version: "0.1.0", configDir: "C:\\Users\\<user>\\AppData\\Roaming\\dev.aeolus.app", dataDir: "C:\\Users\\<user>\\AppData\\Local\\dev.aeolus.app", logsDir: "…\\logs", backupsDir: "…\\backups", installRoot: "C:\\Users\\<user>\\AppData\\Local\\dev.aeolus.app\\Roblox\\Versions", startupNotes: [], platform: "windows", operation: null };
      case "app_ready":
        return null;
      case "get_settings":
        return structuredClone(s.settings);
      case "save_settings":
        if (!/^#[0-9a-fA-F]{6}$/.test(args.settings.general.accent)) return err("Accent color must be a hex color");
        s.settings = structuredClone(args.settings);
        return structuredClone(s.settings);
      case "reset_settings":
        return s.settings;
      case "system_integration":
        return { discordAvailable: true, robloxRunning: false, launchAtStartup: s.settings.general.launchAtStartup, protocol: { supported: true, handlers: [], registeredToAeolus: false } };
      case "activity_list":
        return s.activity;
      case "activity_clear":
        s.activity = [];
        return null;
      case "install_overview": {
        const i = s.install;
        return {
          state: i,
          update: { installed: i.active?.version ?? null, installedGuid: i.active?.versionGuid ?? null, latest: i.lastCheck.latest?.version ?? null, latestGuid: i.lastCheck.latest?.versionGuid ?? null, updateAvailable: !!i.active && i.active.versionGuid !== i.lastCheck.latest?.versionGuid, checkedAt: i.lastCheck.checkedAt, error: null },
          detection: {
            installs: [
              { id: "1", source: "aeolus", sourceLabel: "Aeolus", path: i.active?.path ?? "", versionGuid: i.active?.versionGuid ?? "", version: i.active?.version ?? null, managed: true, valid: true, issues: [], detectedAt: now() },
              { id: "2", source: "official", sourceLabel: "Roblox (official)", path: "C:\\Users\\<user>\\AppData\\Local\\Roblox\\Versions\\version-2d7e90aa1f6c4b33", versionGuid: "version-2d7e90aa1f6c4b33", version: null, managed: false, valid: true, issues: [], detectedAt: now() },
            ].filter((x) => x.path),
            incomplete: [],
            scanned: ["a", "b"],
            detectedAt: now(),
          },
          installRoot: "C:\\Users\\<user>\\AppData\\Local\\dev.aeolus.app\\Roblox\\Versions",
          operation: null,
          launchTarget: i.active?.path ?? null,
        };
      }
      case "update_history":
        return [
          { id: "u1", kind: "update", fromVersion: "0.700.0.7000312", toVersion: "0.701.0.7010921", at: ago(1440), result: "success", error: null, durationSeconds: 48.2 },
          { id: "u2", kind: "update", fromVersion: "0.700.0.7000312", toVersion: "0.701.0.7010921", at: ago(3000), result: "failed", error: "Network error: connection reset", durationSeconds: 12 },
        ];
      case "launch_history":
        return s.launches;
      case "launch_roblox": {
        const r = { id: String(Date.now()), at: now(), version: s.install.active?.version, installPath: null, profile: "Default", fastFlagProfile: null, mode: "app", result: "started", error: null, suggestions: [], pid: 4242 };
        s.launches.unshift(r);
        return r;
      }
      case "check_for_updates":
        return { installed: s.install.active?.version, latest: s.install.lastCheck.latest.version, updateAvailable: true, checkedAt: now() };
      case "ff_profiles":
        return s.flags.map((f) => ({ id: f.id, name: f.name, description: f.description, count: Object.keys(f.flags).length, modifiedAt: f.modifiedAt, isDefault: s.settings.fastFlags.defaultProfileId === f.id }));
      case "ff_rows":
        return ff(args.profileId) ? rows(ff(args.profileId)) : err("not found", "not_found");
      case "ff_set_flag": {
        const p = ff(args.profileId);
        if (typeOf(args.name) === "unknown") return err("Unrecognized prefix");
        if (args.previousName && args.previousName !== args.name) delete p.flags[args.previousName];
        p.flags[args.name] = args.value;
        return { status: ALLOW.includes(args.name) ? "allowlisted" : "notAllowlisted", messages: [] };
      }
      case "ff_remove_flags": {
        const p = ff(args.profileId);
        for (const n of args.names) delete p.flags[n];
        return args.names.length;
      }
      case "ff_create": {
        const id = "f" + (s.flags.length + 10);
        s.flags.push({ id, name: args.name, description: "", flags: {}, modifiedAt: now() });
        return { id };
      }
      case "ff_import": {
        const obj = JSON.parse(args.json);
        Object.assign(ff(args.profileId).flags, obj);
        return { imported: Object.keys(obj).length, rejected: [] };
      }
      case "ff_export":
        return JSON.stringify(ff(args.profileId).flags, null, 2);
      case "ff_allowlist":
        return ALLOW.map((a) => [a, "graphics"]);
      case "ff_plugin_presets":
        return [];
      case "ff_compare":
        return { onlyLeft: [], onlyRight: [], changed: [], same: 0 };
      case "profiles_get":
        return s.profiles;
      case "profile_set_active":
        s.profiles.activeId = args.id;
        return null;
      case "profile_create": {
        const p = { ...args.input, id: "p" + Date.now(), createdAt: now(), lastUsedAt: null };
        s.profiles.profiles.push(p);
        return p;
      }
      case "diagnostics_system":
        return { osName: "Windows 11 Home 24H2", osVersion: "11 (26100)", osBuild: "26100", architecture: "x86_64", cpu: "AMD Ryzen 7 7800X3D 8-Core Processor", cpuCores: 16, gpus: ["NVIDIA GeForce RTX 4070"], totalMemory: 34_359_738_368, availableMemory: 19_100_000_000 };
      case "logs_query":
        return s.logs.filter((l) => !args.query.levels || args.query.levels.includes(l.level)).filter((l) => !args.query.search || l.message.toLowerCase().includes(args.query.search.toLowerCase()));
      case "plugins_list":
        return s.plugins;
      case "backups_overview":
        return [{ id: "20261111-101500-000__before-edit", category: "fastflags", label: "before-edit", createdAt: ago(55), size: 2048 }];
      default:
        return null;
    }
  };
}

export function installMock(state: MockState = createState()) {
  mockWindows("main");
  (window as any).__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
  const h = handler(state);
  mockIPC((cmd, args) => h(cmd, args as Any));
  return state;
}
