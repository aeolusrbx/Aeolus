import { useEffect, useState, type ReactNode } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Copy, Download, RefreshCw, RotateCcw, Trash2, Upload } from "lucide-react";
import { api } from "../services/api";
import { useApp } from "../stores/app";
import { useResource } from "../hooks/useResource";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import { useRestoreDialog } from "../components/RestoreDialog";
import { NAV } from "../components/Sidebar";
import { Logo } from "../components/Logo";
import { Button, Card, Field, Input, PageHeader, Select, Toggle, cx } from "../components/ui";
import type { Settings as S } from "../types";

const SECTIONS = ["General", "Installation", "Launch", "Fast Flags", "Plugins", "Privacy", "Advanced", "About"] as const;
type Section = (typeof SECTIONS)[number];

function NumberSetting({ value, min, max, onCommit, suffix }: { value: number; min: number; max: number; onCommit: (n: number) => void; suffix?: string }) {
  const [v, setV] = useState(String(value));
  useEffect(() => setV(String(value)), [value]);
  const n = Number(v);
  const bad = !/^\d+$/.test(v) || n < min || n > max;
  return (
    <div className="flex items-center gap-2">
      <Input className={cx("w-20 text-right", bad && "border-danger/60")} value={v} onChange={(e) => setV(e.target.value)} onBlur={() => (bad ? setV(String(value)) : n !== value && onCommit(n))} aria-label="Number" />
      {suffix && <span className="text-xs text-fg-3">{suffix}</span>}
    </div>
  );
}

function Group({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mb-6">
      <h2 className="mb-2 text-[11px] font-semibold uppercase tracking-wider text-fg-3">{title}</h2>
      <Card className="divide-y divide-line px-4">{children}</Card>
    </div>
  );
}

export default function Settings() {
  const { settings, updateSettings, info, navigate } = useApp();
  const [section, setSection] = useState<Section>("General");
  const profiles = useResource(api.profilesGet, ["profiles"]);
  const ffProfiles = useResource(api.ffProfiles, ["fastflags"]);
  const integration = useResource(api.systemIntegration, ["settings"]);
  const dialogs = useDialogs();
  const toast = useToast();
  const openRestore = useRestoreDialog();
  const [feed, setFeed] = useState(settings?.advanced.selfUpdateFeed ?? "");
  const [checkingSelf, setCheckingSelf] = useState(false);
  useEffect(() => setFeed(settings?.advanced.selfUpdateFeed ?? ""), [settings?.advanced.selfUpdateFeed]);
  if (!settings) return null;
  const s = settings;
  const set = <K extends keyof S>(sec: K, patch: Partial<S[K]>) => updateSettings((cur) => ({ ...cur, [sec]: { ...(cur[sec] as object), ...patch } }));
  const copy = (t: string) => navigator.clipboard.writeText(t).then(() => toast.info("Copied", t));

  const pathRow = (label: string, path?: string) => (
    <Field label={label} description={<span className="selectable break-all font-mono">{path ?? "…"}</span>}>
      {path && <Button size="sm" variant="ghost" icon={<Copy size={12} />} onClick={() => copy(path)}>Copy</Button>}
    </Field>
  );

  return (
    <div className="mx-auto flex max-w-5xl gap-8 px-8 py-8">
      <nav className="sticky top-0 w-40 shrink-0 self-start" aria-label="Settings sections">
        <h1 className="mb-4 text-xl font-semibold tracking-tight text-fg">Settings</h1>
        {SECTIONS.map((x) => (
          <button key={x} onClick={() => setSection(x)} className={cx("mb-0.5 block w-full rounded-lg px-3 py-1.5 text-left text-[13px] transition-colors", section === x ? "bg-surface font-medium text-fg ring-1 ring-line" : "text-fg-2 hover:text-fg")}>
            {x}
          </button>
        ))}
      </nav>

      <div className="min-w-0 flex-1">
        <PageHeader title={section} />

        {section === "General" && (
          <>
            <Group title="Appearance">
              <Field label="Theme">
                <Select className="w-40" value={s.general.theme} onChange={(e) => set("general", { theme: e.target.value as S["general"]["theme"] })}>
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                  <option value="system">System default</option>
                </Select>
              </Field>
              <Field label="Accent color" description="Pick presets or a custom color on the Customization page.">
                <span className="h-5 w-5 rounded-full border border-line" style={{ background: s.general.accent }} />
                <Button size="sm" onClick={() => navigate("customization")}>Customize</Button>
              </Field>
              <Field label="Language" description="Aeolus is currently available in English only.">
                <Select className="w-40" value={s.general.language} onChange={(e) => set("general", { language: e.target.value })}>
                  <option value="en">English</option>
                </Select>
              </Field>
            </Group>
            <Group title="Startup">
              <Field label="Launch at Windows startup" description="Registers Aeolus under your user's startup programs (HKCU Run). Turning this off removes the entry.">
                <Toggle checked={s.general.launchAtStartup} onChange={(v) => set("general", { launchAtStartup: v })} />
              </Field>
              <Field label="Start minimized" description="Start in the system tray.">
                <Toggle checked={s.general.startMinimized} onChange={(v) => set("general", { startMinimized: v })} />
              </Field>
              <Field label="Default page">
                <Select className="w-44" value={s.general.defaultPage} onChange={(e) => set("general", { defaultPage: e.target.value as S["general"]["defaultPage"] })}>
                  {NAV.map((n) => <option key={n.id} value={n.id}>{n.label}</option>)}
                </Select>
              </Field>
              {integration.data && (
                <Field label="Startup registration status">
                  <span className={cx("text-xs", integration.data.launchAtStartup ? "text-success" : "text-fg-3")}>{integration.data.launchAtStartup ? "Registered" : "Not registered"}</span>
                </Field>
              )}
            </Group>
            <Group title="Sidebar">
              <Field label="Collapsed sidebar"><Toggle checked={s.general.sidebarCollapsed} onChange={(v) => set("general", { sidebarCollapsed: v })} /></Field>
              <Field label="Compact navigation"><Toggle checked={s.appearance.compactNavigation} onChange={(v) => set("appearance", { compactNavigation: v })} /></Field>
              <Field label="Close to system tray" description="The close button hides Aeolus instead of exiting."><Toggle checked={s.general.minimizeToTray} onChange={(v) => set("general", { minimizeToTray: v })} /></Field>
            </Group>
          </>
        )}

        {section === "Installation" && (
          <>
            <Group title="Installation directory">
              <Field label="Aeolus installs Roblox in" description={<span className="selectable break-all font-mono">{info?.installRoot}</span>}>
                <Button size="sm" onClick={() => navigate("installations")}>Change</Button>
              </Field>
            </Group>
            <Group title="Detection">
              <Field label="Detect the official Roblox installation"><Toggle checked={s.installation.scanOfficialInstall} onChange={(v) => set("installation", { scanOfficialInstall: v })} /></Field>
              <Field label="Detect other bootstrappers" description="Bloxstrap and Fishstrap installation folders."><Toggle checked={s.installation.scanOtherBootstrappers} onChange={(v) => set("installation", { scanOtherBootstrappers: v })} /></Field>
            </Group>
            <Group title="Backups">
              <Field label="Back up before destructive operations" description="Reinstall, repair and uninstall back up settings, profiles and Fast Flags first."><Toggle checked={s.installation.backupBeforeDestructive} onChange={(v) => set("installation", { backupBeforeDestructive: v })} /></Field>
              <Field label="Backups to keep per type"><NumberSetting value={s.installation.keepBackups} min={1} max={100} onCommit={(n) => set("installation", { keepBackups: n })} /></Field>
            </Group>
            <Group title="Updates">
              <Field label="Check for Roblox updates when Aeolus starts"><Toggle checked={s.installation.checkUpdatesOnStartup} onChange={(v) => set("installation", { checkUpdatesOnStartup: v })} /></Field>
              <Field label="Install updates before launching" description="Profiles can also turn this off individually."><Toggle checked={s.installation.updateBeforeLaunch} onChange={(v) => set("installation", { updateBeforeLaunch: v })} /></Field>
              <Field label="Keep downloaded packages" description="Uses more disk space but makes repairs faster."><Toggle checked={s.installation.keepDownloadCache} onChange={(v) => set("installation", { keepDownloadCache: v })} /></Field>
            </Group>
          </>
        )}

        {section === "Launch" && (
          <>
            <Group title="Launch">
              <Field label="Default profile" description="Used when no profile is selected.">
                <Select className="w-48" value={profiles.data?.defaultId ?? ""} onChange={(e) => api.profileSetDefault(e.target.value).then(() => toast.success("Default profile set")).catch((err) => dialogs.showError("Could not set default", err))}>
                  {profiles.data?.profiles.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
                </Select>
              </Field>
              <Field label="Launch preferences" description="Fast Flags, auto-update, verification and after-launch behavior are set per profile.">
                <Button size="sm" onClick={() => navigate("launch")}>Edit profiles</Button>
              </Field>
              <Field label="Handle Roblox links" description="Open roblox-player: links from the Roblox website with Aeolus, so your profile and Fast Flags apply. The previous handler is restored when turned off.">
                <Toggle checked={s.advanced.handleRobloxLinks} disabled={integration.data ? !integration.data.protocol.supported : false} onChange={(v) => set("advanced", { handleRobloxLinks: v })} />
              </Field>
            </Group>
            <Group title="Notifications">
              <Field label="Windows notifications" description="Use native notifications in addition to in-app messages."><Toggle checked={s.notifications.native} onChange={(v) => set("notifications", { native: v })} /></Field>
              <Field label="Update available"><Toggle checked={s.notifications.updateAvailable} onChange={(v) => set("notifications", { updateAvailable: v })} /></Field>
              <Field label="Update completed"><Toggle checked={s.notifications.updateCompleted} onChange={(v) => set("notifications", { updateCompleted: v })} /></Field>
              <Field label="Update failed"><Toggle checked={s.notifications.updateFailed} onChange={(v) => set("notifications", { updateFailed: v })} /></Field>
              <Field label="Roblox version changed"><Toggle checked={s.notifications.versionChanged} onChange={(v) => set("notifications", { versionChanged: v })} /></Field>
              <Field label="Launch failed"><Toggle checked={s.notifications.launchFailed} onChange={(v) => set("notifications", { launchFailed: v })} /></Field>
            </Group>
          </>
        )}

        {section === "Fast Flags" && (
          <Group title="Fast Flags">
            <Field label="Default Fast Flag profile" description="Used by launch profiles that don't select one.">
              <Select className="w-48" value={s.fastFlags.defaultProfileId ?? ""} onChange={(e) => api.ffSetDefault(e.target.value || null).catch((err) => dialogs.showError("Could not set default", err))}>
                <option value="">None (no overrides)</option>
                {ffProfiles.data?.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}
              </Select>
            </Field>
            <Field label="Back up on every change" description="A copy of all Fast Flag profiles is saved before each edit."><Toggle checked={s.fastFlags.backupOnChange} onChange={(v) => set("fastFlags", { backupOnChange: v })} /></Field>
            <Field label="Backups to keep"><NumberSetting value={s.fastFlags.maxBackups} min={1} max={200} onCommit={(n) => set("fastFlags", { maxBackups: n })} /></Field>
            <Field label="Block invalid values" description="Reject values whose type doesn't match the flag name (e.g. text for an FInt)."><Toggle checked={s.fastFlags.blockInvalidValues} onChange={(v) => set("fastFlags", { blockInvalidValues: v })} /></Field>
            <Field label="Warn about non-allowlisted flags"><Toggle checked={s.fastFlags.warnNotAllowlisted} onChange={(v) => set("fastFlags", { warnNotAllowlisted: v })} /></Field>
          </Group>
        )}

        {section === "Plugins" && (
          <Group title="Plugins">
            <Field label="Require re-approval when a plugin changes" description="A plugin whose manifest changes is disabled until you approve it again."><Toggle checked={s.plugins.requireReapprovalOnChange} onChange={(v) => set("plugins", { requireReapprovalOnChange: v })} /></Field>
            <Field label="Show plugin notifications" description="Allow approved plugins to show Windows notifications."><Toggle checked={s.notifications.pluginEvents} onChange={(v) => set("notifications", { pluginEvents: v })} /></Field>
            <Field label="Manage plugins"><Button size="sm" onClick={() => navigate("plugins")}>Open Plugins</Button></Field>
          </Group>
        )}

        {section === "Privacy" && (
          <>
            <Group title="Diagnostics">
              <Field label="Include logs in diagnostic reports" description="Logs are redacted: cookies, tokens and your Windows user name are removed."><Toggle checked={s.privacy.includeLogsInReports} onChange={(v) => set("privacy", { includeLogsInReports: v })} /></Field>
              <Field label="Keep logs for"><NumberSetting value={s.privacy.logRetentionDays} min={1} max={365} suffix="days" onCommit={(n) => set("privacy", { logRetentionDays: n })} /></Field>
            </Group>
            <Group title="Your data">
              <Field label="Export all data" description="Settings, profiles, Fast Flags and history as one JSON file.">
                <Button size="sm" icon={<Download size={12} />} onClick={async () => {
                  const f = await saveDialog({ defaultPath: "aeolus-export.json", filters: [{ name: "JSON", extensions: ["json"] }] }).catch(() => null);
                  if (f) api.exportAllData(f).then(() => toast.success("Data exported", f)).catch((e) => dialogs.showError("Export failed", e));
                }}>Export</Button>
              </Field>
              <Field label="Import data" description="Replaces settings, profiles and Fast Flags. Current data is backed up first.">
                <Button size="sm" icon={<Upload size={12} />} onClick={async () => {
                  const f = await openDialog({ filters: [{ name: "JSON", extensions: ["json"] }] }).catch(() => null);
                  if (typeof f !== "string") return;
                  const ok = await dialogs.confirm({ title: "Import configuration?", description: "Your current settings, profiles and Fast Flags will be replaced. They are backed up first and can be restored.", confirmLabel: "Import", tone: "warning" });
                  if (ok) api.importAllData(f).then(() => toast.success("Configuration imported")).catch((e) => dialogs.showError("Import failed", e));
                }}>Import</Button>
              </Field>
              <Field label="Restore a backup"><Button size="sm" icon={<RotateCcw size={12} />} onClick={() => openRestore()}>Restore</Button></Field>
              <Field label="Delete data" description="Clears logs, activity and history. Optionally resets all configuration too.">
                <Button size="sm" variant="danger" icon={<Trash2 size={12} />} onClick={async () => {
                  const r = await dialogs.confirm({ title: "Delete your Aeolus data?", description: "Logs, recent activity, launch history and update history are permanently deleted. Roblox itself is not affected.", confirmLabel: "Delete", tone: "danger", checkbox: { label: "Also reset settings, profiles and Fast Flags (a backup is kept)" } });
                  if (r) api.deleteUserData(r.checked).then(() => toast.success("Data deleted")).catch((e) => dialogs.showError("Could not delete data", e));
                }}>Delete…</Button>
              </Field>
            </Group>
          </>
        )}

        {section === "Advanced" && (
          <>
            <Group title="Developer">
              <Field label="Developer mode" description="Shows internal identifiers in the interface."><Toggle checked={s.advanced.developerMode} onChange={(v) => set("advanced", { developerMode: v })} /></Field>
              <Field label="Debug logging" description="Records DEBUG entries. Useful when reporting a problem."><Toggle checked={s.advanced.debugLogging} onChange={(v) => set("advanced", { debugLogging: v })} /></Field>
            </Group>
            <Group title="Locations">
              {pathRow("Configuration directory", info?.configDir)}
              {pathRow("Data directory", info?.dataDir)}
              {pathRow("Logs", info?.logsDir)}
              {pathRow("Backups", info?.backupsDir)}
            </Group>
            <Group title="Aeolus updates">
              <Field label="Release feed URL" description="Where Aeolus looks for new versions. Defaults to the official GitHub releases. Leave empty to disable.">
                <Input className="w-72" placeholder="https://…/aeolus-latest.json" value={feed} onChange={(e) => setFeed(e.target.value)} onBlur={() => feed !== s.advanced.selfUpdateFeed && set("advanced", { selfUpdateFeed: feed.trim() })} />
              </Field>
              <Field label="Check for Aeolus updates on startup" description="Shows a notification when a newer Aeolus is published.">
                <Toggle checked={s.advanced.checkAeolusUpdatesOnStartup} onChange={(v) => set("advanced", { checkAeolusUpdatesOnStartup: v })} />
              </Field>
              <Field label="Check for Aeolus updates">
                <Button size="sm" icon={<RefreshCw size={12} />} loading={checkingSelf} onClick={async () => {
                  setCheckingSelf(true);
                  try {
                    const r = await api.selfUpdateCheck();
                    if (!r.configured) toast.info("No release feed configured", "Add a feed URL above to enable update checks.");
                    else if (!r.updateAvailable) toast.success("Aeolus is up to date", `Version ${r.current}`);
                    else {
                      const ok = await dialogs.confirm({ title: `Aeolus ${r.latest!.version} is available`, description: (<><div>{r.latest!.notes || "No release notes."}</div><div className="mt-2 text-xs text-fg-3">The installer is downloaded and its SHA-256 checksum verified. Aeolus then closes and the installer runs; your configuration is kept.</div></>), confirmLabel: "Download & install" });
                      if (ok) await api.selfUpdateInstall();
                    }
                  } catch (e) {
                    dialogs.showError("Update check failed", e);
                  } finally {
                    setCheckingSelf(false);
                  }
                }}>Check now</Button>
              </Field>
            </Group>
            <Group title="Reset">
              <Field label="Reset all settings" description="Restores default settings. Profiles and Fast Flags are kept; the current settings are backed up.">
                <Button size="sm" variant="danger" onClick={async () => {
                  const ok = await dialogs.confirm({ title: "Reset settings to defaults?", description: "Profiles and Fast Flags are not affected. A backup of the current settings is kept.", confirmLabel: "Reset", tone: "danger" });
                  if (ok) api.resetSettings().then(() => toast.success("Settings reset")).catch((e) => dialogs.showError("Reset failed", e));
                }}>Reset settings</Button>
              </Field>
            </Group>
          </>
        )}

        {section === "About" && (
          <Card className="p-8 text-center">
            <Logo size={96} className="mx-auto" />
            <div className="mt-4 text-2xl font-semibold tracking-tight text-fg">Aeolus</div>
            <div className="mt-1 text-[13px] text-fg-2">Version {info?.version}</div>
            <p className="mx-auto mt-4 max-w-md text-[13px] text-fg-2">
              A Roblox bootstrapper and client manager. Aeolus installs and updates Roblox through Roblox's official distribution, and never asks for your password or account cookies.
            </p>
            <p className="mx-auto mt-3 max-w-md text-xs text-fg-3">Aeolus is an independent project and is not affiliated with or endorsed by Roblox Corporation.</p>
          </Card>
        )}
      </div>
    </div>
  );
}
