import { motion } from "framer-motion";
import {
  Activity as ActivityIcon, AlertCircle, ArchiveRestore, Blocks, CheckCircle2, Download, Flag, HardDrive,
  Play, RefreshCw, Rocket, Settings2, Stethoscope, Trash2, User, Wrench, type LucideIcon,
} from "lucide-react";
import { api } from "../services/api";
import { useApp } from "../stores/app";
import { useResource } from "../hooks/useResource";
import { useRobloxActions } from "../hooks/useRobloxActions";
import { useRestoreDialog } from "../components/RestoreDialog";
import { useDialogs } from "../stores/dialogs";
import { Logo } from "../components/Logo";
import { ProgressView } from "../components/ProgressDock";
import { GameRow } from "../components/GameHistory";
import { Badge, Button, Card, EmptyState, SectionTitle, cx } from "../components/ui";
import type { ActivityKind } from "../types";
import { relativeTime } from "../utils/format";

const ACTIVITY_ICONS: Record<ActivityKind, LucideIcon> = {
  launch: Rocket,
  update: RefreshCw,
  install: Download,
  config: Settings2,
  backup: ArchiveRestore,
  plugin: Blocks,
  error: AlertCircle,
};

function StatusCard({ icon: Icon, label, value, sub, tone }: { icon: LucideIcon; label: string; value: string; sub?: string; tone?: "ok" | "warn" | "bad" }) {
  return (
    <Card className="p-4">
      <div className="flex items-center gap-2 text-xs text-fg-3">
        <Icon size={14} />
        {label}
      </div>
      <div className={cx("mt-2 truncate text-[15px] font-semibold", tone === "warn" ? "text-warning" : tone === "bad" ? "text-danger" : "text-fg")}>{value}</div>
      {sub && <div className="mt-0.5 truncate text-xs text-fg-3">{sub}</div>}
    </Card>
  );
}

export default function Home() {
  const { navigate, settings, progress } = useApp();
  const overview = useResource(api.installOverview, ["install", "operation"]);
  const profiles = useResource(api.profilesGet, ["profiles"]);
  const activity = useResource(api.activityList, ["activity", "install", "launch", "fastflags", "profiles", "plugins", "settings", "all"]);
  const games = useResource(api.gamesOverview, ["games"]);
  const actions = useRobloxActions();
  const openRestore = useRestoreDialog();
  const dialogs = useDialogs();

  const o = overview.data;
  const installed = o?.state.active ?? null;
  const external = o?.state.preferredExternal ?? null;
  const ready = !!(installed || external);
  const busy = !!progress && !progress.done;
  const activeProfile = profiles.data?.profiles.find((p) => p.id === profiles.data?.activeId) ?? profiles.data?.profiles[0];

  const statusText = busy
    ? progress!.operation
    : !o
      ? "Checking…"
      : !ready
        ? "Roblox is not installed"
        : o.update.updateAvailable
          ? "Update available"
          : "Ready to play";

  const bg = settings?.appearance.dashboardBackground ?? "glow";

  return (
    <div className={cx("relative min-h-full", bg === "glow" && "bg-glow")}>
      {bg === "grid" && <div className="bg-grid pointer-events-none absolute inset-x-0 top-0 h-80" />}
      <div className="relative mx-auto max-w-5xl px-8 py-8">
        {/* Welcome + launch */}
        <motion.div initial={{ opacity: 0, y: 6 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: 0.25 }}>
          <Card className="overflow-hidden p-6">
            <div className="flex items-center gap-6">
              <Logo size={72} animated={settings?.appearance.animations === "enabled" && !busy} className="shrink-0" />
              <div className="min-w-0 flex-1">
                <div className="text-xs font-medium uppercase tracking-wider text-accent">Aeolus</div>
                <h1 className="mt-1 text-2xl font-semibold tracking-tight text-fg">Welcome back</h1>
                <div className="mt-1.5 flex flex-wrap items-center gap-2 text-[13px] text-fg-2">
                  <span className={cx("inline-flex items-center gap-1.5", ready && !o?.update.updateAvailable && !busy && "text-success")}>
                    {ready && !o?.update.updateAvailable && !busy ? <CheckCircle2 size={14} /> : <ActivityIcon size={14} />}
                    {statusText}
                  </span>
                  {installed && <span className="text-fg-3">· Roblox {installed.version}</span>}
                  {activeProfile && <span className="text-fg-3">· Profile: {activeProfile.name}</span>}
                </div>
              </div>
              <div className="flex shrink-0 flex-col items-end gap-2">
                {ready ? (
                  <Button size="lg" variant="primary" icon={<Play size={16} fill="currentColor" />} loading={actions.launching} disabled={busy} onClick={() => actions.launch()}>
                    {o?.update.updateAvailable && settings?.installation.updateBeforeLaunch ? "Update & Launch" : "Launch Roblox"}
                  </Button>
                ) : (
                  <Button size="lg" variant="primary" icon={<Download size={16} />} disabled={busy || !o} onClick={() => actions.runInstall("install")}>
                    Install Roblox
                  </Button>
                )}
                {o?.update.checkedAt && <span className="text-[11px] text-fg-3">Checked {relativeTime(o.update.checkedAt)}</span>}
              </div>
            </div>
            {progress && (
              <div className="mt-5 border-t border-line pt-4">
                <ProgressView />
              </div>
            )}
            {o?.state.lastCheck.error && !busy && (
              <div className="mt-4 rounded-lg border border-warning/30 bg-warning/5 px-3 py-2 text-xs text-warning">
                Last update check failed: {o.state.lastCheck.error}
              </div>
            )}
          </Card>
        </motion.div>

        {games.data?.current && (
          <Card className="mt-4 overflow-hidden border-success/30">
            <GameRow game={games.data.current} live />
          </Card>
        )}

        {/* Status cards */}
        <div className="mt-5 grid grid-cols-4 gap-3">
          <StatusCard icon={HardDrive} label="Installation" value={installed ? "Installed" : external ? "External" : "Not installed"} sub={installed ? `${installed.fileCount.toLocaleString()} files` : external ? "Using another installation" : "Install from Aeolus"} tone={ready ? "ok" : "warn"} />
          <StatusCard icon={Wrench} label="Client version" value={installed?.version ?? o?.update.installed ?? "—"} sub={installed?.versionGuid} />
          <StatusCard
            icon={RefreshCw}
            label="Updates"
            value={!o ? "—" : o.update.latest == null ? "Not checked" : o.update.updateAvailable ? "Update available" : installed ? "Up to date" : "—"}
            sub={o?.update.latest ? `Latest ${o.update.latest}` : undefined}
            tone={o?.update.updateAvailable ? "warn" : undefined}
          />
          <StatusCard icon={User} label="Active profile" value={activeProfile?.name ?? "—"} sub={activeProfile?.description || undefined} />
        </div>

        <div className="mt-6 grid grid-cols-5 gap-5">
          {/* Quick actions */}
          <div className="col-span-2">
            <SectionTitle>Quick actions</SectionTitle>
            <div className="grid grid-cols-2 gap-2">
              {[
                { icon: Rocket, label: "Launch Roblox", onClick: () => actions.launch(), disabled: busy || !ready },
                { icon: RefreshCw, label: "Check for updates", onClick: () => actions.checkUpdates(), disabled: actions.checking },
                { icon: HardDrive, label: "Installations", onClick: () => navigate("installations") },
                { icon: Flag, label: "Fast Flags", onClick: () => navigate("fastflags") },
                { icon: Stethoscope, label: "Diagnostics", onClick: () => navigate("diagnostics") },
                { icon: ArchiveRestore, label: "Restore config", onClick: () => openRestore() },
              ].map((a) => (
                <button
                  key={a.label}
                  onClick={a.onClick}
                  disabled={a.disabled}
                  className="group flex items-center gap-2.5 rounded-xl border border-line bg-surface px-3.5 py-3 text-left text-[13px] font-medium text-fg transition-colors hover:border-accent/40 hover:bg-surface-hover disabled:opacity-50"
                >
                  <a.icon size={16} className="text-fg-3 transition-colors group-hover:text-accent" />
                  {a.label}
                </button>
              ))}
            </div>
          </div>

          {/* Recent activity */}
          <div className="col-span-3">
            <SectionTitle
              action={
                activity.data && activity.data.length > 0 ? (
                  <Button
                    size="sm"
                    variant="ghost"
                    icon={<Trash2 size={12} />}
                    onClick={async () => {
                      const ok = await dialogs.confirm({ title: "Clear recent activity?", description: "This only clears the activity list. Logs and history are kept.", confirmLabel: "Clear", tone: "danger" });
                      if (ok) await api.activityClear().then(activity.reload);
                    }}
                  >
                    Clear
                  </Button>
                ) : undefined
              }
            >
              Recent activity
            </SectionTitle>
            <Card className="max-h-[300px] overflow-auto">
              {activity.data?.length === 0 && <EmptyState icon={<ActivityIcon size={22} />} title="No activity yet" description="Launches, updates and configuration changes will appear here." />}
              {activity.data?.slice(0, 40).map((a) => {
                const Icon = ACTIVITY_ICONS[a.kind] ?? ActivityIcon;
                return (
                  <div key={a.id} className="flex items-start gap-3 border-b border-line px-4 py-2.5 last:border-0">
                    <Icon size={15} className={cx("mt-0.5 shrink-0", a.kind === "error" ? "text-danger" : "text-fg-3")} />
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[13px] text-fg">{a.title}</div>
                      {a.detail && <div className="selectable truncate text-xs text-fg-3">{a.detail}</div>}
                    </div>
                    <span className="shrink-0 text-[11px] text-fg-3" title={new Date(a.at).toLocaleString()}>{relativeTime(a.at)}</span>
                  </div>
                );
              })}
            </Card>
          </div>
        </div>
        {games.data && games.data.history.filter((g) => g.id !== games.data!.current?.id).length > 0 && (
          <div className="mt-6">
            <SectionTitle action={<Button size="sm" variant="ghost" onClick={() => navigate("launch")}>See all</Button>}>Recent games</SectionTitle>
            <Card className="divide-y divide-line">
              {games.data.history.filter((g) => g.id !== games.data!.current?.id).slice(0, 3).map((g) => <GameRow key={g.id} game={g} compact />)}
            </Card>
          </div>
        )}
        {o?.detection.incomplete && o.detection.incomplete.length > 0 && !busy && (
          <Card className="mt-5 flex items-center gap-3 border-warning/30 p-4">
            <AlertCircle size={18} className="text-warning" />
            <div className="flex-1 text-[13px] text-fg-2">An interrupted installation was found. Completed downloads are kept, so resuming continues where it stopped.</div>
            <Badge tone="warning">Incomplete</Badge>
            <Button size="sm" variant="primary" onClick={() => actions.runInstall(installed ? "update" : "install")}>Resume</Button>
          </Card>
        )}
      </div>
    </div>
  );
}
