// Types mirroring the Rust backend's serialized structures.

export type Theme = "dark" | "light" | "system";
export type Animations = "enabled" | "reduced" | "disabled";
export type PageId =
  | "home"
  | "installations"
  | "launch"
  | "fastflags"
  | "mods"
  | "customization"
  | "plugins"
  | "diagnostics"
  | "settings";

export interface Settings {
  schemaVersion: number;
  general: {
    theme: Theme;
    accent: string;
    language: string;
    launchAtStartup: boolean;
    startMinimized: boolean;
    minimizeToTray: boolean;
    defaultPage: PageId;
    sidebarCollapsed: boolean;
  };
  appearance: {
    animations: Animations;
    compactNavigation: boolean;
    dashboardBackground: "plain" | "glow" | "grid";
    logoPresentation: "full" | "icon";
    sidebarStyle: "solid" | "subtle";
  };
  installation: {
    customInstallRoot: string | null;
    scanOfficialInstall: boolean;
    scanOtherBootstrappers: boolean;
    backupBeforeDestructive: boolean;
    keepBackups: number;
    checkUpdatesOnStartup: boolean;
    updateBeforeLaunch: boolean;
    keepDownloadCache: boolean;
  };
  launch: { defaultProfileId: string | null; fpsCap: number | null };
  fastFlags: {
    defaultProfileId: string | null;
    backupOnChange: boolean;
    maxBackups: number;
    warnNotAllowlisted: boolean;
    blockInvalidValues: boolean;
  };
  plugins: { requireReapprovalOnChange: boolean; notifyOnErrors: boolean };
  notifications: {
    native: boolean;
    updateAvailable: boolean;
    updateCompleted: boolean;
    updateFailed: boolean;
    versionChanged: boolean;
    launchFailed: boolean;
    pluginEvents: boolean;
  };
  discord: { richPresence: boolean; showProfile: boolean; showGame: boolean };
  games: { trackGames: boolean; serverLocation: boolean; notifyLocation: boolean; rejoin: RejoinMode };
  mods: { skyPackId: string | null };
  privacy: { logRetentionDays: number; includeLogsInReports: boolean };
  advanced: {
    developerMode: boolean;
    debugLogging: boolean;
    handleRobloxLinks: boolean;
    selfUpdateFeed: string;
    checkAeolusUpdatesOnStartup: boolean;
  };
}

export interface AppError {
  kind: string;
  message: string;
  hint?: string | null;
}

export interface AppInfo {
  name: string;
  version: string;
  configDir: string;
  dataDir: string;
  logsDir: string;
  backupsDir: string;
  installRoot: string;
  startupNotes: string[];
  platform: string;
  operation: string | null;
}

export interface Progress {
  operation: string;
  stage: string;
  detail: string | null;
  downloaded: number;
  total: number | null;
  bytesPerSecond: number | null;
  etaSeconds: number | null;
  step: number;
  steps: number;
  done: boolean;
}

export interface ClientVersion {
  version: string;
  versionGuid: string;
}

export interface InstalledVersion {
  version: string;
  versionGuid: string;
  path: string;
  installedAt: string;
  fileCount: number;
  size: number;
}

export interface InstallState {
  schemaVersion: number;
  active: InstalledVersion | null;
  preferredExternal: string | null;
  lastCheck: { checkedAt: string | null; latest: ClientVersion | null; error: string | null };
}

export interface UpdateStatus {
  installed: string | null;
  installedGuid: string | null;
  latest: string | null;
  latestGuid: string | null;
  updateAvailable: boolean;
  checkedAt: string | null;
  error: string | null;
}

export interface DetectedInstall {
  id: string;
  source: string;
  sourceLabel: string;
  path: string;
  versionGuid: string;
  version: string | null;
  managed: boolean;
  valid: boolean;
  issues: string[];
  detectedAt: string;
}

export interface Overview {
  state: InstallState;
  update: UpdateStatus;
  detection: { installs: DetectedInstall[]; incomplete: string[]; scanned: string[]; detectedAt: string };
  installRoot: string;
  operation: string | null;
  launchTarget: string | null;
}

export interface InstallOutcome {
  installed: InstalledVersion;
  previous: InstalledVersion | null;
  warnings: string[];
  downloadedBytes: number;
}

export interface VerifyReport {
  checked: number;
  missing: string[];
  modified: string[];
  indexMissing: boolean;
  overlaid: number;
}

export interface UpdateRecord {
  id: string;
  kind: string;
  fromVersion: string | null;
  toVersion: string;
  at: string;
  result: "success" | "failed" | "cancelled";
  error: string | null;
  durationSeconds: number;
}

export interface LaunchRecord {
  id: string;
  at: string;
  version: string | null;
  installPath: string | null;
  profile: string | null;
  fastFlagProfile: string | null;
  mode: string;
  result: "started" | "failed";
  error: string | null;
  suggestions: string[];
  pid: number | null;
}

export type ActivityKind = "launch" | "update" | "install" | "config" | "backup" | "plugin" | "error";
export interface ActivityEntry {
  id: string;
  kind: ActivityKind;
  title: string;
  detail: string | null;
  at: string;
}

export type FlagCategory = "rendering" | "graphics" | "performance" | "networking" | "interface" | "experimental" | "other";
export type FlagType = "boolean" | "integer" | "string" | "log" | "unknown";
export type FlagStatus = "allowlisted" | "notAllowlisted" | "invalid";

export interface FlagRow {
  name: string;
  value: unknown;
  flagType: FlagType;
  category: FlagCategory;
  status: FlagStatus;
  messages: string[];
}

export interface FlagProfileSummary {
  id: string;
  name: string;
  description: string;
  count: number;
  modifiedAt: string;
  isDefault: boolean;
}

export interface Comparison {
  onlyLeft: string[];
  onlyRight: string[];
  changed: [string, unknown, unknown][];
  same: number;
}

export interface FlagPreset {
  name: string;
  description: string;
  flags: Record<string, unknown>;
}

export interface PluginPreset {
  pluginId: string;
  pluginName: string;
  preset: FlagPreset;
}

export type AfterLaunch = "nothing" | "minimize" | "close";
export interface LaunchPrefs {
  applyFastFlags: boolean;
  updateBeforeLaunch: boolean;
  verifyBeforeLaunch: boolean;
  applyMods: boolean;
  fpsCap?: number | null;
  afterLaunch: AfterLaunch;
}

export interface Profile {
  id: string;
  name: string;
  description: string;
  icon: string;
  accent: string;
  fastFlagProfileId: string | null;
  /** null = global sky, "default" = Roblox's sky, else a sky pack id */
  sky: string | null;
  launch: LaunchPrefs;
  createdAt: string;
  lastUsedAt: string | null;
}

export interface ProfileStore {
  schemaVersion: number;
  profiles: Profile[];
  defaultId: string | null;
  activeId: string | null;
}

export type ProfileInput = Omit<Profile, "id" | "createdAt" | "lastUsedAt">;

export type LogLevel = "DEBUG" | "INFO" | "WARNING" | "ERROR";
export interface LogEntry {
  ts: string;
  level: LogLevel;
  source: string;
  message: string;
}
export interface LogQuery {
  search?: string;
  levels?: LogLevel[];
  from?: string;
  to?: string;
  limit?: number;
}

export interface SystemInfo {
  osName: string;
  osVersion: string;
  osBuild: string;
  architecture: string;
  cpu: string;
  cpuCores: number;
  gpus: string[];
  totalMemory: number;
  availableMemory: number;
}

export type PluginPermission = "log" | "notifications" | "fastflags.presets" | "launch.hooks";
export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  homepage?: string | null;
  aeolusApi: number;
  permissions: PluginPermission[];
  dependencies: { id: string; minVersion?: string | null }[];
  contributes: {
    fastFlagPresets: FlagPreset[];
    configuration: { key: string; label: string; type: "boolean" | "string" | "integer"; default: unknown; description: string }[];
    hooks: Record<string, unknown[]>;
  };
}
export interface PluginInfo {
  id: string;
  manifest: PluginManifest | null;
  status: "enabled" | "disabled" | "needs-approval" | "changed" | "error";
  error: string | null;
  permissions: [PluginPermission, string][];
  dependenciesOk: boolean;
  dependencyIssues: string[];
  config: Record<string, unknown>;
  path: string;
  lastUpdated: string | null;
}

export interface BackupEntry {
  id: string;
  category: string;
  label: string;
  createdAt: string;
  size: number;
}

export interface SystemIntegration {
  discordAvailable: boolean;
  robloxRunning: boolean;
  launchAtStartup: boolean;
  protocol: { supported: boolean; handlers: [string, string | null][]; registeredToAeolus: boolean };
}

export interface SelfUpdateCheck {
  current: string;
  latest: { version: string; url: string; sha256: string; notes: string } | null;
  updateAvailable: boolean;
  configured: boolean;
}

export interface SkyPack {
  id: string;
  name: string;
  files: string[];
  createdAt: string;
  preview: string | null;
  previewUrl: string | null;
}

export interface SkiesOverview {
  packs: SkyPack[];
  globalPackId: string | null;
  appliedPackId: string | null;
  appliedPackName: string | null;
}

export type ModKind = "sound" | "cursor" | "font";
export interface ModChoice {
  enabled: boolean;
  file: string | null;
  originalName: string | null;
  preset: string | null;
  updatedAt: string;
}
export interface ModSlot {
  id: string;
  kind: ModKind;
  label: string;
  description: string;
  targets: string[];
  presets: { id: string; label: string }[];
  choice: ModChoice | null;
  previewUrl: string | null;
  presetPreviews: [string, string | null][];
}

export interface GameVisit {
  id: string;
  placeId: number;
  universeId: number | null;
  jobId: string;
  serverKind: "public" | "private" | "reserved";
  startedAt: string;
  endedAt: string | null;
  name: string | null;
  creator: string | null;
  iconUrl: string | null;
  location: string | null;
}
export interface GamesOverview {
  current: GameVisit | null;
  history: GameVisit[];
  tracking: boolean;
  disconnect: Disconnect | null;
  crash: CrashReport | null;
}
export type RejoinMode = "off" | "ask" | "auto";
export interface Disconnect {
  id: string;
  at: string;
  game: GameVisit;
  code: number | null;
  title: string;
  detail: string;
  canRejoin: boolean;
  sameServer: boolean;
  autoAt: string | null;
}
export interface CrashReport {
  id: string;
  at: string;
  exitCode: string;
  title: string;
  explanation: string;
  suggestions: string[];
  game: string | null;
}
