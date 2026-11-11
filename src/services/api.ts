// Typed wrappers around the Rust commands.
import { invoke } from "@tauri-apps/api/core";
import type * as T from "../types";

export const inTauri = (): boolean =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e && typeof e === "object" && "message" in e) return String((e as T.AppError).message);
  return "Unexpected error";
}

export function errorHint(e: unknown): string | null {
  if (e && typeof e === "object" && "hint" in e) return (e as T.AppError).hint ?? null;
  return null;
}

export function isCancelled(e: unknown): boolean {
  return !!e && typeof e === "object" && (e as T.AppError).kind === "cancelled";
}

async function call<R>(cmd: string, args?: Record<string, unknown>): Promise<R> {
  if (!inTauri()) {
    throw { kind: "unavailable", message: "The Aeolus backend is not available in a plain browser.", hint: null } satisfies T.AppError;
  }
  return invoke<R>(cmd, args);
}

export const api = {
  // app
  appInfo: () => call<T.AppInfo>("app_info"),
  appReady: () => call<void>("app_ready"),
  getSettings: () => call<T.Settings>("get_settings"),
  saveSettings: (settings: T.Settings) => call<T.Settings>("save_settings", { settings }),
  resetSettings: () => call<T.Settings>("reset_settings"),
  systemIntegration: () => call<T.SystemIntegration>("system_integration"),
  activityList: () => call<T.ActivityEntry[]>("activity_list"),
  activityClear: () => call<void>("activity_clear"),
  selfUpdateCheck: () => call<T.SelfUpdateCheck>("self_update_check"),
  selfUpdateInstall: () => call<void>("self_update_install"),
  cancelOperation: () => call<boolean>("cancel_operation"),
  closeWindow: () => call<void>("close_window"),
  exportAllData: (path: string) => call<void>("export_all_data", { path }),
  importAllData: (path: string) => call<void>("import_all_data", { path }),
  deleteUserData: (includeConfig: boolean) => call<void>("delete_user_data", { includeConfig }),

  // installation
  installOverview: () => call<T.Overview>("install_overview"),
  checkForUpdates: () => call<T.UpdateStatus>("check_for_updates"),
  runInstall: (kind: "install" | "update" | "repair" | "reinstall") => call<T.InstallOutcome>("run_install_operation", { kind }),
  verifyInstallation: (full: boolean) => call<T.VerifyReport>("verify_installation", { full }),
  uninstallRoblox: (clearCache: boolean) => call<void>("uninstall_roblox", { clearCache }),
  discardIncomplete: (versionGuid: string) => call<void>("discard_incomplete", { versionGuid }),
  clearDownloadCache: () => call<number>("clear_download_cache"),
  setLaunchInstall: (path: string | null) => call<void>("set_launch_install", { path }),
  setInstallRoot: (path: string | null) => call<void>("set_install_root", { path }),
  updateHistory: () => call<T.UpdateRecord[]>("update_history"),
  launchRoblox: (profileId?: string | null) => call<T.LaunchRecord>("launch_roblox", { profileId: profileId ?? null }),
  launchHistory: () => call<T.LaunchRecord[]>("launch_history"),
  clearLaunchHistory: () => call<void>("clear_launch_history"),

  // fast flags
  ffProfiles: () => call<T.FlagProfileSummary[]>("ff_profiles"),
  ffRows: (profileId: string) => call<T.FlagRow[]>("ff_rows", { profileId }),
  ffCreate: (name: string, copyFrom: string | null) => call<{ id: string }>("ff_create", { name, copyFrom }),
  ffDuplicate: (profileId: string) => call<{ id: string }>("ff_duplicate", { profileId }),
  ffRename: (profileId: string, name: string, description: string | null) => call<void>("ff_rename", { profileId, name, description }),
  ffDelete: (profileId: string) => call<void>("ff_delete", { profileId }),
  ffSetFlag: (profileId: string, name: string, value: unknown, previousName: string | null) =>
    call<{ status: T.FlagStatus; messages: string[] }>("ff_set_flag", { profileId, name, value, previousName }),
  ffRemoveFlags: (profileId: string, names: string[]) => call<number>("ff_remove_flags", { profileId, names }),
  ffReset: (profileId: string) => call<void>("ff_reset", { profileId }),
  ffImport: (profileId: string, json: string, replace: boolean) => call<{ imported: number; rejected: string[] }>("ff_import", { profileId, json, replace }),
  ffImportFile: (profileId: string, path: string, replace: boolean) => call<{ imported: number; rejected: string[] }>("ff_import_file", { profileId, path, replace }),
  ffExport: (profileId: string) => call<string>("ff_export", { profileId }),
  ffExportFile: (profileId: string, path: string) => call<void>("ff_export_file", { profileId, path }),
  ffCompare: (left: string, right: string) => call<T.Comparison>("ff_compare", { left, right }),
  ffSetDefault: (profileId: string | null) => call<void>("ff_set_default", { profileId }),
  ffApplyNow: (profileId: string) => call<number>("ff_apply_now", { profileId }),
  ffPluginPresets: () => call<T.PluginPreset[]>("ff_plugin_presets"),
  ffAllowlist: () => call<[string, T.FlagCategory][]>("ff_allowlist"),

  // profiles
  profilesGet: () => call<T.ProfileStore>("profiles_get"),
  profileCreate: (input: T.ProfileInput) => call<T.Profile>("profile_create", { input }),
  profileUpdate: (id: string, input: T.ProfileInput) => call<T.Profile>("profile_update", { id, input }),
  profileDuplicate: (id: string) => call<T.Profile>("profile_duplicate", { id }),
  profileDelete: (id: string) => call<void>("profile_delete", { id }),
  profileSetDefault: (id: string) => call<void>("profile_set_default", { id }),
  profileSetActive: (id: string) => call<void>("profile_set_active", { id }),
  profileExportFile: (id: string, path: string) => call<void>("profile_export_file", { id, path }),
  profileImportFile: (path: string) => call<T.Profile>("profile_import_file", { path }),

  // skies
  skiesOverview: () => call<T.SkiesOverview>("skies_overview"),
  skyImport: (path: string, name: string | null) => call<T.SkyPack>("sky_import", { path, name }),
  skyRename: (id: string, name: string) => call<void>("sky_rename", { id, name }),
  skyDelete: (id: string) => call<void>("sky_delete", { id }),
  skySetGlobal: (id: string | null) => call<string | null>("sky_set_global", { id }),

  // mods
  modsOverview: () => call<T.ModSlot[]>("mods_overview"),
  modSetFile: (slot: string, path: string) => call<void>("mod_set_file", { slot, path }),
  modSetPreset: (slot: string, preset: string) => call<void>("mod_set_preset", { slot, preset }),
  modSetEnabled: (slot: string, enabled: boolean) => call<void>("mod_set_enabled", { slot, enabled }),
  modClear: (slot: string) => call<void>("mod_clear", { slot }),
  modsResetAll: () => call<void>("mods_reset_all"),

  // backups
  backupsOverview: () => call<T.BackupEntry[]>("backups_overview"),
  restoreBackup: (category: string, id: string) => call<void>("restore_backup", { category, id }),
  createManualBackup: () => call<number>("create_manual_backup"),

  // diagnostics
  diagnosticsSystem: () => call<T.SystemInfo>("diagnostics_system"),
  logsQuery: (query: T.LogQuery) => call<T.LogEntry[]>("logs_query", { query }),
  logsClear: () => call<number>("logs_clear"),
  logsExport: (query: T.LogQuery, path: string) => call<number>("logs_export", { query, path }),
  diagnosticsReport: (path: string, display: string | null) => call<void>("diagnostics_report", { path, display }),

  // plugins
  pluginsList: () => call<T.PluginInfo[]>("plugins_list"),
  pluginEnable: (id: string) => call<void>("plugin_enable", { id }),
  pluginDisable: (id: string) => call<void>("plugin_disable", { id }),
  pluginSetConfig: (id: string, key: string, value: unknown) => call<void>("plugin_set_config", { id, key, value }),
  pluginInstall: (path: string) => call<string>("plugin_install", { path }),
  pluginUninstall: (id: string) => call<void>("plugin_uninstall", { id }),
};

export const EVENTS = {
  progress: "aeolus://progress",
  changed: "aeolus://changed",
  navigate: "aeolus://navigate",
  toast: "aeolus://toast",
} as const;
