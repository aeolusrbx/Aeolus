import { save as saveDialog, open as openDialog } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Check, Copy, Download, MoreHorizontal, Pencil, Play, Plus, Star, Trash2, Upload } from "lucide-react";
import { useState } from "react";
import { api } from "../services/api";
import { useResource } from "../hooks/useResource";
import { useRobloxActions } from "../hooks/useRobloxActions";
import { useApp } from "../stores/app";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import { ProfileEditor, ProfileIcon } from "../components/ProfileEditor";
import { Badge, Button, Card, EmptyState, IconButton, PageHeader, SectionTitle, cx } from "../components/ui";
import type { Profile, ProfileInput } from "../types";
import { formatDate, relativeTime } from "../utils/format";

export default function Launch() {
  const { navigate, progress, settings } = useApp();
  const store = useResource(api.profilesGet, ["profiles"]);
  const flags = useResource(api.ffProfiles, ["fastflags"]);
  const history = useResource(api.launchHistory, ["launch"]);
  const ov = useResource(api.installOverview, ["install"]);
  const skies = useResource(api.skiesOverview, ["skies"]);
  const actions = useRobloxActions();
  const dialogs = useDialogs();
  const toast = useToast();
  const [menu, setMenu] = useState<string | null>(null);

  const busy = !!progress && !progress.done;
  const s = store.data;
  const activeId = s?.activeId ?? s?.defaultId ?? null;
  const lastFailure = history.data?.[0]?.result === "failed" ? history.data[0] : null;

  const edit = (p?: Profile) =>
    dialogs.custom<void>((close) => (
      <ProfileEditor
        initial={p}
        flagProfiles={flags.data ?? []}
        skies={skies.data?.packs ?? []}
        onClose={() => close(null)}
        onSave={async (input: ProfileInput) => {
          try {
            if (p) await api.profileUpdate(p.id, input);
            else await api.profileCreate(input);
            toast.success(p ? "Profile saved" : "Profile created", input.name);
            return true;
          } catch (e) {
            dialogs.showError("Could not save profile", e);
            return false;
          }
        }}
      />
    ));

  const act = async (fn: () => Promise<unknown>, ok?: string) => {
    setMenu(null);
    try {
      await fn();
      if (ok) toast.success(ok);
    } catch (e) {
      dialogs.showError("Action failed", e);
    }
  };

  const flagName = (id: string | null) => (id ? flags.data?.find((f) => f.id === id)?.name ?? "Missing profile" : "Default Fast Flags");

  return (
    <div className="mx-auto max-w-5xl px-8 py-8">
      <PageHeader
        title="Launch"
        description="Launch Roblox with a profile. Profiles combine launch preferences with a Fast Flag configuration."
        actions={
          <>
            <Button
              icon={<Upload size={15} />}
              onClick={async () => {
                const f = await openDialog({ filters: [{ name: "Aeolus profile", extensions: ["json"] }] }).catch(() => null);
                if (typeof f === "string") act(() => api.profileImportFile(f), "Profile imported");
              }}
            >
              Import
            </Button>
            <Button variant="primary" icon={<Plus size={15} />} onClick={() => edit()}>New profile</Button>
          </>
        }
      />

      <Card className="mb-6 flex items-center gap-4 p-5">
        <div className="min-w-0 flex-1">
          <div className="text-[13px] text-fg-2">Launching with</div>
          <div className="mt-0.5 flex items-center gap-2 text-base font-semibold text-fg">
            {s && activeId && <ProfileIcon name={s.profiles.find((p) => p.id === activeId)?.icon ?? "wind"} className="text-accent" />}
            {s?.profiles.find((p) => p.id === activeId)?.name ?? "—"}
          </div>
          <div className="mt-1 truncate text-xs text-fg-3">
            {ov.data?.launchTarget ? `Installation: ${ov.data.launchTarget}` : "No installation available — install Roblox first"}
          </div>
        </div>
        <Button size="lg" variant="primary" icon={<Play size={16} fill="currentColor" />} loading={actions.launching} disabled={busy} onClick={() => actions.launch(activeId)}>
          Launch Roblox
        </Button>
      </Card>

      {lastFailure && (
        <Card className="mb-6 border-danger/30 p-4">
          <div className="flex items-start gap-3">
            <AlertTriangle size={18} className="mt-0.5 shrink-0 text-danger" />
            <div className="min-w-0 flex-1">
              <div className="text-[13px] font-semibold text-fg">The last launch failed ({relativeTime(lastFailure.at)})</div>
              <div className="selectable mt-1 text-[13px] text-fg-2">{lastFailure.error}</div>
              <ul className="mt-2 list-disc space-y-0.5 pl-5 text-xs text-fg-2">
                {lastFailure.suggestions.map((s) => <li key={s}>{s}</li>)}
              </ul>
            </div>
            <Button size="sm" onClick={() => navigate("diagnostics")}>Open Diagnostics</Button>
          </div>
        </Card>
      )}

      <SectionTitle>Profiles</SectionTitle>
      <div className="grid grid-cols-2 gap-3">
        {s?.profiles.map((p) => {
          const isActive = p.id === activeId;
          return (
            <Card key={p.id} className={cx("relative p-4 transition-colors", isActive && "border-accent/50")}>
              <div className="flex items-start gap-3">
                <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg" style={{ background: p.accent + "22", color: p.accent }}>
                  <ProfileIcon name={p.icon} size={17} />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-1.5">
                    <span className="truncate text-[14px] font-semibold text-fg">{p.name}</span>
                    {s.defaultId === p.id && <Badge tone="accent"><Star size={10} /> Default</Badge>}
                  </div>
                  {p.description && <div className="truncate text-xs text-fg-2">{p.description}</div>}
                  <div className="mt-2 flex flex-wrap gap-1.5">
                    <Badge>{flagName(p.fastFlagProfileId)}</Badge>
                    {p.sky && <Badge>{p.sky === "default" ? "Roblox sky" : `Sky: ${skies.data?.packs.find((s) => s.id === p.sky)?.name ?? "missing"}`}</Badge>}
                    {p.launch.updateBeforeLaunch && <Badge>Auto-update</Badge>}
                    {p.launch.verifyBeforeLaunch && <Badge>Verify</Badge>}
                    {p.launch.afterLaunch !== "nothing" && <Badge>{p.launch.afterLaunch === "close" ? "Close after launch" : "Minimize after launch"}</Badge>}
                  </div>
                  <div className="mt-2 text-[11px] text-fg-3">Created {formatDate(p.createdAt)} · Last used {relativeTime(p.lastUsedAt)}</div>
                  {settings?.advanced.developerMode && <div className="selectable mt-0.5 font-mono text-[10px] text-fg-3">id {p.id}{p.fastFlagProfileId ? ` · flags ${p.fastFlagProfileId}` : ""}</div>}
                </div>
                <div className="relative">
                  <IconButton label="Profile actions" onClick={() => setMenu(menu === p.id ? null : p.id)}><MoreHorizontal size={16} /></IconButton>
                  {menu === p.id && (
                    <>
                      <div className="fixed inset-0 z-10" onClick={() => setMenu(null)} />
                      <div className="absolute right-0 top-9 z-20 w-48 overflow-hidden rounded-xl border border-line bg-surface py-1 shadow-[var(--shadow)]">
                        {[
                          { icon: Pencil, label: "Edit / rename", on: () => { setMenu(null); edit(p); } },
                          { icon: Copy, label: "Duplicate", on: () => act(() => api.profileDuplicate(p.id), "Profile duplicated") },
                          { icon: Star, label: "Set as default", on: () => act(() => api.profileSetDefault(p.id), "Default profile set") },
                          {
                            icon: Download,
                            label: "Export…",
                            on: async () => {
                              setMenu(null);
                              const f = await saveDialog({ defaultPath: `${p.name}.aeolus-profile.json`, filters: [{ name: "JSON", extensions: ["json"] }] }).catch(() => null);
                              if (f) act(() => api.profileExportFile(p.id, f), "Profile exported");
                            },
                          },
                          {
                            icon: Trash2,
                            label: "Delete",
                            danger: true,
                            on: async () => {
                              setMenu(null);
                              const ok = await dialogs.confirm({ title: `Delete "${p.name}"?`, description: "The profile is removed. Its Fast Flag profile is kept. A backup of your profiles is made first.", confirmLabel: "Delete", tone: "danger" });
                              if (ok) act(() => api.profileDelete(p.id), "Profile deleted");
                            },
                          },
                        ].map((m) => (
                          <button key={m.label} onClick={m.on} className={cx("flex w-full items-center gap-2.5 px-3 py-2 text-left text-[13px] hover:bg-surface-hover", m.danger ? "text-danger" : "text-fg")}>
                            <m.icon size={14} /> {m.label}
                          </button>
                        ))}
                      </div>
                    </>
                  )}
                </div>
              </div>
              <div className="mt-3 flex gap-2 border-t border-line pt-3">
                {isActive ? (
                  <span className="inline-flex h-7 items-center gap-1.5 px-1 text-xs font-medium text-accent"><Check size={13} /> Active</span>
                ) : (
                  <Button size="sm" onClick={() => act(() => api.profileSetActive(p.id))}>Make active</Button>
                )}
                <Button size="sm" variant="ghost" icon={<Play size={12} />} disabled={busy || actions.launching} onClick={() => actions.launch(p.id)}>Launch with this</Button>
              </div>
            </Card>
          );
        })}
      </div>

      <div className="mt-8">
        <SectionTitle
          action={
            history.data && history.data.length > 0 ? (
              <Button size="sm" variant="ghost" onClick={async () => (await dialogs.confirm({ title: "Clear launch history?", confirmLabel: "Clear", tone: "danger" })) && act(() => api.clearLaunchHistory())}>
                Clear
              </Button>
            ) : undefined
          }
        >
          Launch history
        </SectionTitle>
        <Card className="overflow-hidden">
          {history.data?.length === 0 && <EmptyState title="No launches yet" description="Every launch is recorded here with its result." />}
          {history.data && history.data.length > 0 && (
            <table className="w-full text-left text-[13px]">
              <thead className="border-b border-line text-xs text-fg-3">
                <tr>
                  <th className="px-4 py-2 font-medium">Time</th>
                  <th className="px-4 py-2 font-medium">Roblox version</th>
                  <th className="px-4 py-2 font-medium">Profile</th>
                  <th className="px-4 py-2 font-medium">Fast Flags</th>
                  <th className="px-4 py-2 font-medium">Result</th>
                </tr>
              </thead>
              <tbody>
                {history.data.slice(0, 30).map((h) => (
                  <tr key={h.id} className="border-b border-line last:border-0">
                    <td className="whitespace-nowrap px-4 py-2 text-fg-2">{formatDate(h.at)}</td>
                    <td className="px-4 py-2 font-mono text-xs text-fg-2">{h.version ?? "—"}</td>
                    <td className="px-4 py-2 text-fg">{h.profile ?? "—"}{h.mode === "link" && <Badge className="ml-1.5">Link</Badge>}</td>
                    <td className="px-4 py-2 text-fg-2">{h.fastFlagProfile ?? "—"}</td>
                    <td className="max-w-[260px] px-4 py-2">
                      {h.result === "started" ? <span className="text-xs text-success">Started</span> : <span className="selectable block truncate text-xs text-danger" title={h.error ?? ""}>Failed: {h.error}</span>}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </Card>
      </div>
    </div>
  );
}
