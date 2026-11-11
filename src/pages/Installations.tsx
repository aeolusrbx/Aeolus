import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { CheckCircle2, Copy, Download, FolderOpen, HardDrive, History, RefreshCw, ScanSearch, ShieldCheck, Trash2, Wrench, XCircle } from "lucide-react";
import { api } from "../services/api";
import { useResource } from "../hooks/useResource";
import { useRobloxActions } from "../hooks/useRobloxActions";
import { useApp } from "../stores/app";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import { ProgressView } from "../components/ProgressDock";
import { Badge, Button, Card, EmptyState, PageHeader, SectionTitle, Spinner, cx } from "../components/ui";
import { formatBytes, formatDate, formatDuration, relativeTime } from "../utils/format";
import type { VerifyReport } from "../types";

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex gap-4 py-1.5 text-[13px]">
      <div className="w-28 shrink-0 text-fg-3">{label}</div>
      <div className="selectable min-w-0 flex-1 break-all text-fg">{children}</div>
    </div>
  );
}

export default function Installations() {
  const { progress } = useApp();
  const ov = useResource(api.installOverview, ["install", "operation", "settings"]);
  const hist = useResource(api.updateHistory, ["install"]);
  const actions = useRobloxActions();
  const dialogs = useDialogs();
  const toast = useToast();
  const [verifying, setVerifying] = useState(false);
  const [lastVerify, setLastVerify] = useState<VerifyReport | null>(null);

  const o = ov.data;
  const active = o?.state.active ?? null;
  const busy = !!progress && !progress.done;

  const verify = async (full: boolean) => {
    setVerifying(true);
    try {
      const r = await api.verifyInstallation(full);
      setLastVerify(r);
      const ok = !r.indexMissing && r.missing.length === 0 && r.modified.length === 0;
      if (ok) toast.success("Installation verified", `All ${r.checked.toLocaleString()} files are intact.`);
      else {
        const go = await dialogs.confirm({
          title: "Problems found",
          description: `${r.missing.length} missing and ${r.modified.length} damaged files${r.indexMissing ? " (file index missing)" : ""}. Repair re-downloads the affected version and replaces the installation once it verifies.`,
          confirmLabel: "Repair now",
          tone: "warning",
        });
        if (go) await actions.runInstall("repair");
      }
    } catch (e) {
      dialogs.showError("Verification failed", e);
    } finally {
      setVerifying(false);
    }
  };

  const uninstall = async () => {
    const r = await dialogs.confirm({
      title: "Uninstall Roblox?",
      description: (
        <>
          This deletes the Aeolus-managed Roblox files at <span className="selectable break-all font-mono text-xs text-fg">{active?.path}</span>. Your settings, profiles, Fast Flags and Roblox account data are not affected, and other Roblox installations are left alone.
        </>
      ),
      confirmLabel: "Uninstall",
      tone: "danger",
      checkbox: { label: "Also clear the download cache" },
    });
    if (!r) return;
    try {
      await api.uninstallRoblox(r.checked);
      toast.success("Roblox uninstalled", "Configuration was kept.");
    } catch (e) {
      dialogs.showError("Uninstall failed", e);
    }
  };

  const changeRoot = async () => {
    const dir = await openDialog({ directory: true, title: "Choose where Aeolus installs Roblox" }).catch(() => null);
    if (typeof dir !== "string") return;
    try {
      await api.setInstallRoot(dir);
      toast.success("Installation folder changed", "Roblox will be installed there on the next install or update.");
    } catch (e) {
      dialogs.showError("Folder not usable", e);
    }
  };

  return (
    <div className="mx-auto max-w-5xl px-8 py-8">
      <PageHeader
        title="Installations"
        description="Install, update and maintain the Roblox Player using Roblox's official distribution."
        actions={
          <>
            <Button icon={<ScanSearch size={15} />} onClick={() => ov.reload()} loading={ov.loading && !!o}>Detect</Button>
            <Button icon={<RefreshCw size={15} />} loading={actions.checking} onClick={() => actions.checkUpdates()}>Check for updates</Button>
          </>
        }
      />

      {!o && ov.loading && <div className="flex justify-center py-16"><Spinner size={22} /></div>}
      {ov.error != null && !o && <Card className="p-4 text-[13px] text-danger">Could not read installation status.</Card>}

      {o && (
        <>
          <Card className="p-5">
            <div className="flex items-start gap-4">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-accent/12 text-accent"><HardDrive size={19} /></div>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <h2 className="text-[15px] font-semibold text-fg">Aeolus installation</h2>
                  {active ? (o.update.updateAvailable ? <Badge tone="warning">Update available</Badge> : o.update.latest ? <Badge tone="success">Up to date</Badge> : <Badge>Installed</Badge>) : <Badge tone="neutral">Not installed</Badge>}
                  {!o.state.preferredExternal && active && <Badge tone="accent">Used for launching</Badge>}
                </div>
                {active ? (
                  <div className="mt-2">
                    <Row label="Version">{active.version} <span className="text-fg-3">({active.versionGuid})</span></Row>
                    <Row label="Location">{active.path}</Row>
                    <Row label="Installed">{formatDate(active.installedAt)}</Row>
                    <Row label="Size">{formatBytes(active.size)} · {active.fileCount.toLocaleString()} files</Row>
                    {o.update.latest && <Row label="Latest">{o.update.latest} <span className="text-fg-3">· checked {relativeTime(o.update.checkedAt)}</span></Row>}
                  </div>
                ) : (
                  <p className="mt-1 text-[13px] text-fg-2">Aeolus keeps its own copy of the Roblox Player so updates and Fast Flags never interfere with other launchers.</p>
                )}
              </div>
            </div>

            <div className="mt-4 flex flex-wrap gap-2 border-t border-line pt-4">
              {!active && <Button variant="primary" icon={<Download size={15} />} disabled={busy} onClick={() => actions.runInstall("install")}>Install Roblox</Button>}
              {active && o.update.updateAvailable && <Button variant="primary" icon={<Download size={15} />} disabled={busy} onClick={() => actions.runInstall("update")}>Update to {o.update.latest}</Button>}
              {active && (
                <>
                  <Button icon={<ShieldCheck size={15} />} disabled={busy} loading={verifying} onClick={() => verify(false)}>Quick verify</Button>
                  <Button icon={<ShieldCheck size={15} />} disabled={busy || verifying} onClick={() => verify(true)}>Full verify</Button>
                  <Button icon={<Wrench size={15} />} disabled={busy} onClick={() => actions.runInstall("repair")}>Repair</Button>
                  <Button icon={<RefreshCw size={15} />} disabled={busy} onClick={() => actions.runInstall("reinstall")}>Reinstall</Button>
                  <div className="flex-1" />
                  <Button variant="danger" icon={<Trash2 size={15} />} disabled={busy} onClick={uninstall}>Uninstall</Button>
                </>
              )}
            </div>
            {lastVerify && (
              <div className="mt-3 text-xs text-fg-3">
                Last verification: {lastVerify.checked.toLocaleString()} files checked, {lastVerify.missing.length} missing, {lastVerify.modified.length} damaged{lastVerify.overlaid ? `, ${lastVerify.overlaid} replaced by your custom sky` : ""}.
              </div>
            )}
            {progress && (
              <div className="mt-4 rounded-lg border border-line bg-bg-2 p-3.5">
                <ProgressView />
              </div>
            )}
          </Card>

          {o.detection.incomplete.length > 0 && (
            <Card className="mt-4 p-4">
              <SectionTitle>Interrupted installations</SectionTitle>
              {o.detection.incomplete.map((g) => (
                <div key={g} className="flex items-center gap-3 py-1.5">
                  <XCircle size={15} className="text-warning" />
                  <span className="flex-1 font-mono text-xs text-fg-2">{g}</span>
                  <Button size="sm" variant="primary" disabled={busy} onClick={() => actions.runInstall(active ? "update" : "install")}>Resume</Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    disabled={busy}
                    onClick={async () => {
                      const ok = await dialogs.confirm({ title: "Discard interrupted installation?", description: "The partially extracted files are deleted. Verified downloads in the cache are kept.", confirmLabel: "Discard", tone: "danger" });
                      if (ok) await api.discardIncomplete(g).catch((e) => dialogs.showError("Could not discard", e));
                    }}
                  >
                    Discard
                  </Button>
                </div>
              ))}
            </Card>
          )}

          <div className="mt-6">
            <SectionTitle
              action={
                <div className="flex gap-1">
                  <Button size="sm" variant="ghost" icon={<FolderOpen size={13} />} onClick={changeRoot}>Change folder</Button>
                  {o.state && (
                    <Button size="sm" variant="ghost" onClick={() => api.setInstallRoot(null).then(() => toast.success("Using the default folder")).catch((e) => dialogs.showError("Could not reset", e))}>
                      Use default
                    </Button>
                  )}
                </div>
              }
            >
              Install location
            </SectionTitle>
            <Card className="flex items-center gap-3 px-4 py-3">
              <FolderOpen size={15} className="text-fg-3" />
              <span className="selectable flex-1 break-all font-mono text-xs text-fg-2">{o.installRoot}</span>
              <button className="text-fg-3 hover:text-fg" title="Copy path" onClick={() => navigator.clipboard.writeText(o.installRoot).then(() => toast.info("Path copied"))}>
                <Copy size={14} />
              </button>
            </Card>
          </div>

          <div className="mt-6">
            <SectionTitle>Detected installations</SectionTitle>
            <Card>
              {o.detection.installs.length === 0 && <EmptyState icon={<ScanSearch size={22} />} title="No Roblox installations found" description={`Scanned ${o.detection.scanned.length} standard locations.`} />}
              {o.detection.installs.map((i) => {
                const inUse = i.managed ? !o.state.preferredExternal : o.state.preferredExternal === i.path;
                return (
                  <div key={i.id} className="flex items-center gap-3 border-b border-line px-4 py-3 last:border-0">
                    {i.valid ? <CheckCircle2 size={16} className="shrink-0 text-success" /> : <XCircle size={16} className="shrink-0 text-danger" />}
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2 text-[13px] text-fg">
                        {i.sourceLabel}
                        <span className="font-mono text-[11px] text-fg-3">{i.version ?? i.versionGuid}</span>
                        {inUse && <Badge tone="accent">In use</Badge>}
                      </div>
                      <div className="selectable truncate text-xs text-fg-3" title={i.path}>{i.path}</div>
                      {!i.valid && <div className="text-xs text-danger">{i.issues.join(" · ")}</div>}
                    </div>
                    {!inUse && i.valid && (
                      <Button size="sm" disabled={busy} onClick={() => api.setLaunchInstall(i.managed ? null : i.path).then(() => toast.success("Launch installation changed", i.sourceLabel)).catch((e) => dialogs.showError("Could not select installation", e))}>
                        Use for launching
                      </Button>
                    )}
                  </div>
                );
              })}
            </Card>
            <div className="mt-2 flex items-center justify-between text-[11px] text-fg-3">
              <span>Last detection {relativeTime(o.detection.detectedAt)}</span>
              <button
                className="hover:text-fg"
                onClick={async () => {
                  try {
                    const n = await api.clearDownloadCache();
                    toast.success("Download cache cleared", formatBytes(n) + " freed");
                  } catch (e) {
                    dialogs.showError("Could not clear cache", e);
                  }
                }}
              >
                Clear download cache
              </button>
            </div>
          </div>

          <div className="mt-6">
            <SectionTitle>
              <span className="inline-flex items-center gap-1.5"><History size={12} /> Update history</span>
            </SectionTitle>
            <Card className="overflow-hidden">
              {hist.data?.length === 0 && <EmptyState title="No installs or updates yet" />}
              {hist.data && hist.data.length > 0 && (
                <table className="w-full text-left text-[13px]">
                  <thead className="border-b border-line text-xs text-fg-3">
                    <tr>
                      <th className="px-4 py-2 font-medium">Date</th>
                      <th className="px-4 py-2 font-medium">Operation</th>
                      <th className="px-4 py-2 font-medium">Previous</th>
                      <th className="px-4 py-2 font-medium">New</th>
                      <th className="px-4 py-2 font-medium">Result</th>
                    </tr>
                  </thead>
                  <tbody>
                    {hist.data.slice(0, 25).map((h) => (
                      <tr key={h.id} className="border-b border-line last:border-0">
                        <td className="whitespace-nowrap px-4 py-2 text-fg-2">{formatDate(h.at)}</td>
                        <td className="px-4 py-2 capitalize text-fg">{h.kind}</td>
                        <td className="px-4 py-2 font-mono text-xs text-fg-2">{h.fromVersion ?? "—"}</td>
                        <td className="px-4 py-2 font-mono text-xs text-fg-2">{h.toVersion}</td>
                        <td className="px-4 py-2">
                          <span title={h.error ?? undefined} className={cx("text-xs", h.result === "success" ? "text-success" : h.result === "failed" ? "text-danger" : "text-fg-3")}>
                            {h.result === "success" ? `Success · ${formatDuration(h.durationSeconds)}` : h.result === "failed" ? `Failed: ${h.error}` : "Cancelled"}
                          </span>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              )}
            </Card>
          </div>
        </>
      )}
    </div>
  );
}
