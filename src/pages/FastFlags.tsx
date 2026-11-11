import { useMemo, useState } from "react";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  ArrowDownUp, Blocks, ClipboardCopy, Copy, Download, FileUp, GitCompare, Info, Pencil, Plus, RotateCcw, Search, Send, Star, Trash2, Upload,
} from "lucide-react";
import { api, errorMessage } from "../services/api";
import { useResource } from "../hooks/useResource";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import { useApp } from "../stores/app";
import { CompareDialog, FlagEditor, ImportFlagsDialog } from "../components/FlagDialogs";
import { useRestoreDialog } from "../components/RestoreDialog";
import { Badge, Button, Card, EmptyState, IconButton, Input, PageHeader, Select, Spinner, cx } from "../components/ui";
import type { FlagCategory, FlagRow, FlagStatus } from "../types";
import { displayValue } from "../utils/format";

const CATEGORIES: FlagCategory[] = ["rendering", "graphics", "performance", "networking", "interface", "experimental", "other"];
const STATUS_LABEL: Record<FlagStatus, { label: string; tone: "success" | "warning" | "danger" }> = {
  allowlisted: { label: "Allowlisted", tone: "success" },
  notAllowlisted: { label: "Not allowlisted", tone: "warning" },
  invalid: { label: "Invalid", tone: "danger" },
};
type SortKey = "name" | "value" | "flagType" | "category" | "status";

export default function FastFlags() {
  const { settings } = useApp();
  const profiles = useResource(api.ffProfiles, ["fastflags"]);
  const [selected, setSelected] = useState<string | null>(null);
  const pid = selected && profiles.data?.some((p) => p.id === selected) ? selected : profiles.data?.[0]?.id ?? null;
  const rows = useResource(() => (pid ? api.ffRows(pid) : Promise.resolve([] as FlagRow[])), ["fastflags"], [pid]);
  const allow = useResource(api.ffAllowlist, []);
  const presets = useResource(api.ffPluginPresets, ["plugins"]);
  const dialogs = useDialogs();
  const toast = useToast();
  const openRestore = useRestoreDialog();

  const [q, setQ] = useState("");
  const [cat, setCat] = useState<"" | FlagCategory>("");
  const [status, setStatus] = useState<"" | FlagStatus>("");
  const [sort, setSort] = useState<{ key: SortKey; dir: 1 | -1 }>({ key: "name", dir: 1 });
  const [checked, setChecked] = useState<Set<string>>(new Set());

  const profile = profiles.data?.find((p) => p.id === pid);
  const allowNames = useMemo(() => (allow.data ?? []).map(([n]) => n), [allow.data]);

  const visible = useMemo(() => {
    const needle = q.trim().toLowerCase();
    const list = (rows.data ?? []).filter(
      (r) => (!needle || r.name.toLowerCase().includes(needle) || displayValue(r.value).toLowerCase().includes(needle)) && (!cat || r.category === cat) && (!status || r.status === status),
    );
    return list.sort((a, b) => {
      const av = sort.key === "value" ? displayValue(a.value) : String(a[sort.key]);
      const bv = sort.key === "value" ? displayValue(b.value) : String(b[sort.key]);
      return av.localeCompare(bv, undefined, { numeric: true }) * sort.dir;
    });
  }, [rows.data, q, cat, status, sort]);

  const counts = useMemo(() => {
    const c = { allowlisted: 0, notAllowlisted: 0, invalid: 0 };
    for (const r of rows.data ?? []) c[r.status]++;
    return c;
  }, [rows.data]);

  const run = async (fn: () => Promise<unknown>, ok?: string, okDetail?: string) => {
    try {
      await fn();
      if (ok) toast.success(ok, okDetail);
      return true;
    } catch (e) {
      dialogs.showError("Fast Flags", e);
      return false;
    }
  };

  const editFlag = (row?: FlagRow) =>
    pid &&
    dialogs.custom<void>((close) => (
      <FlagEditor
        initialName={row?.name}
        initialValue={row?.value}
        allowlist={allowNames}
        onClose={() => close(null)}
        onSave={async (name, value) => {
          try {
            const c = await api.ffSetFlag(pid, name, value, row?.name ?? null);
            if (c.status === "notAllowlisted" && settings?.fastFlags.warnNotAllowlisted) toast.warning(`${name} saved`, "It is not on Roblox's allowlist, so Roblox will most likely ignore it.");
            return null;
          } catch (e) {
            return errorMessage(e);
          }
        }}
      />
    ));

  const SortHead = ({ k, children, className }: { k: SortKey; children: string; className?: string }) => (
    <th className={cx("px-3 py-2 font-medium", className)}>
      <button className="inline-flex items-center gap-1 hover:text-fg" onClick={() => setSort((s) => ({ key: k, dir: s.key === k ? ((-s.dir) as 1 | -1) : 1 }))}>
        {children}
        {sort.key === k && <ArrowDownUp size={11} className="text-accent" />}
      </button>
    </th>
  );

  return (
    <div className="mx-auto flex h-full max-w-6xl flex-col px-8 py-8">
      <PageHeader
        title="Fast Flags"
        description="Manage ClientAppSettings overrides in separate profiles. Changes are validated and backed up automatically."
        actions={
          <>
            <Button icon={<RotateCcw size={15} />} onClick={() => openRestore()}>Restore</Button>
            <Button icon={<GitCompare size={15} />} disabled={(profiles.data?.length ?? 0) < 2} onClick={() => pid && dialogs.custom<void>((close) => <CompareDialog profiles={profiles.data!} initialLeft={pid} onClose={() => close(null)} />)}>Compare</Button>
          </>
        }
      />

      <div className="mb-4 flex items-start gap-3 rounded-xl border border-line bg-surface px-4 py-3 text-xs text-fg-2">
        <Info size={15} className="mt-0.5 shrink-0 text-accent" />
        <div>
          Since September 2025 Roblox only applies Fast Flags on its <span className="text-fg">official allowlist</span> ({allowNames.length} known flags) and silently ignores the rest. Aeolus validates names and types for every flag, but only allowlisted flags actually take effect. Valid JSON does not mean Roblox supports a flag.
        </div>
      </div>

      <div className="flex min-h-0 flex-1 gap-5">
        {/* profile list */}
        <div className="w-56 shrink-0">
          <div className="mb-2 flex items-center justify-between">
            <span className="text-[11px] font-semibold uppercase tracking-wider text-fg-3">Profiles</span>
            <IconButton
              label="New profile"
              onClick={async () => {
                const name = await dialogs.prompt({ title: "New Fast Flag profile", label: "Name", validate: (v) => (!v.trim() ? "Required" : v.length > 60 ? "Too long" : null), confirmLabel: "Create" });
                if (name) await run(async () => setSelected((await api.ffCreate(name, null)).id), "Profile created");
              }}
            >
              <Plus size={15} />
            </IconButton>
          </div>
          <div className="space-y-0.5">
            {profiles.data?.map((p) => (
              <button
                key={p.id}
                onClick={() => { setSelected(p.id); setChecked(new Set()); }}
                className={cx("flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-left text-[13px] transition-colors", p.id === pid ? "bg-surface text-fg ring-1 ring-line" : "text-fg-2 hover:bg-surface/60 hover:text-fg")}
              >
                <span className="flex-1 truncate">{p.name}</span>
                {p.isDefault && <Star size={11} className="shrink-0 text-accent" fill="currentColor" />}
                <span className="text-[11px] text-fg-3">{p.count}</span>
              </button>
            ))}
          </div>
          {profile && (
            <div className="mt-4 space-y-0.5 border-t border-line pt-3">
              {[
                { icon: Star, label: profile.isDefault ? "Default profile" : "Set as default", disabled: profile.isDefault, on: () => run(() => api.ffSetDefault(profile.id), "Default Fast Flag profile set") },
                {
                  icon: Pencil,
                  label: "Rename",
                  on: async () => {
                    const name = await dialogs.prompt({ title: "Rename profile", initial: profile.name, validate: (v) => (!v.trim() ? "Required" : null) });
                    if (name) await run(() => api.ffRename(profile.id, name, null), "Profile renamed");
                  },
                },
                { icon: Copy, label: "Duplicate", on: () => run(async () => setSelected((await api.ffDuplicate(profile.id)).id), "Profile duplicated") },
                {
                  icon: Download,
                  label: "Export to file",
                  on: async () => {
                    const f = await saveDialog({ defaultPath: `${profile.name}.json`, filters: [{ name: "JSON", extensions: ["json"] }] }).catch(() => null);
                    if (f) await run(() => api.ffExportFile(profile.id, f), "Exported", f);
                  },
                },
                {
                  icon: Trash2,
                  label: "Delete",
                  danger: true,
                  disabled: (profiles.data?.length ?? 0) <= 1,
                  on: async () => {
                    const ok = await dialogs.confirm({ title: `Delete "${profile.name}"?`, description: "Profiles that use it fall back to the default Fast Flags. A backup is kept and can be restored.", confirmLabel: "Delete", tone: "danger" });
                    if (ok) await run(() => api.ffDelete(profile.id), "Profile deleted");
                  },
                },
              ].map((a) => (
                <button key={a.label} disabled={a.disabled} onClick={a.on} className={cx("flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-xs transition-colors disabled:opacity-50", a.danger ? "text-danger hover:bg-danger/10" : "text-fg-2 hover:bg-surface/60 hover:text-fg")}>
                  <a.icon size={13} /> {a.label}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* table */}
        <div className="flex min-w-0 flex-1 flex-col">
          <div className="mb-3 flex flex-wrap items-center gap-2">
            <div className="relative w-56">
              <Search size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-3" />
              <Input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search flags" className="pl-8" aria-label="Search flags" />
            </div>
            <Select value={cat} onChange={(e) => setCat(e.target.value as FlagCategory | "")} className="w-36" aria-label="Category filter">
              <option value="">All categories</option>
              {CATEGORIES.map((c) => <option key={c} value={c}>{c[0].toUpperCase() + c.slice(1)}</option>)}
            </Select>
            <Select value={status} onChange={(e) => setStatus(e.target.value as FlagStatus | "")} className="w-40" aria-label="Status filter">
              <option value="">All statuses</option>
              <option value="allowlisted">Allowlisted ({counts.allowlisted})</option>
              <option value="notAllowlisted">Not allowlisted ({counts.notAllowlisted})</option>
              <option value="invalid">Invalid ({counts.invalid})</option>
            </Select>
            <div className="flex-1" />
            {checked.size > 0 && (
              <Button size="sm" variant="danger" icon={<Trash2 size={13} />} onClick={async () => {
                const ok = await dialogs.confirm({ title: `Delete ${checked.size} flag(s)?`, description: "A backup is made first.", confirmLabel: "Delete", tone: "danger" });
                if (ok && pid) { await run(() => api.ffRemoveFlags(pid, [...checked]), "Flags deleted"); setChecked(new Set()); }
              }}>Delete {checked.size}</Button>
            )}
            <Button size="sm" icon={<Upload size={13} />} disabled={!pid} onClick={() => pid && dialogs.custom<void>((close) => (
              <ImportFlagsDialog onClose={() => close(null)} onImport={async (json, replace) => {
                try {
                  const r = await api.ffImport(pid, json, replace);
                  toast.success(`Imported ${r.imported} flag(s)`, r.rejected.length ? `${r.rejected.length} rejected: ${r.rejected.slice(0, 3).join("; ")}` : undefined);
                  return null;
                } catch (e) { return errorMessage(e); }
              }} />
            ))}>Paste JSON</Button>
            <Button size="sm" icon={<FileUp size={13} />} disabled={!pid} onClick={async () => {
              const f = await openDialog({ filters: [{ name: "JSON", extensions: ["json"] }] }).catch(() => null);
              if (typeof f === "string" && pid) {
                try {
                  const r = await api.ffImportFile(pid, f, false);
                  toast.success(`Imported ${r.imported} flag(s)`, r.rejected.length ? `${r.rejected.length} rejected` : undefined);
                } catch (e) { dialogs.showError("Import failed", e); }
              }
            }}>Import file</Button>
            <Button size="sm" icon={<ClipboardCopy size={13} />} disabled={!pid} onClick={() => pid && run(async () => navigator.clipboard.writeText(await api.ffExport(pid)), "Copied as JSON")}>Copy JSON</Button>
            <Button size="sm" variant="primary" icon={<Plus size={13} />} disabled={!pid} onClick={() => editFlag()}>Add flag</Button>
          </div>

          <Card className="min-h-0 flex-1 overflow-auto">
            {rows.loading && !rows.data && <div className="flex justify-center p-10"><Spinner /></div>}
            {rows.data && rows.data.length === 0 && (
              <EmptyState title="No flags in this profile" description="Add a flag, paste JSON, or import a ClientAppSettings.json file." action={<Button size="sm" variant="primary" icon={<Plus size={13} />} onClick={() => editFlag()}>Add flag</Button>} />
            )}
            {rows.data && rows.data.length > 0 && (
              <table className="w-full text-left text-[13px]">
                <thead className="sticky top-0 z-[1] border-b border-line bg-surface text-xs text-fg-3">
                  <tr>
                    <th className="w-8 px-3 py-2">
                      <input type="checkbox" aria-label="Select all" className="accent-[var(--accent)]" checked={visible.length > 0 && visible.every((r) => checked.has(r.name))} onChange={(e) => setChecked(e.target.checked ? new Set(visible.map((r) => r.name)) : new Set())} />
                    </th>
                    <SortHead k="name">Flag name</SortHead>
                    <SortHead k="value">Value</SortHead>
                    <SortHead k="flagType">Type</SortHead>
                    <SortHead k="category">Category</SortHead>
                    <SortHead k="status">Status</SortHead>
                    <th className="w-20" />
                  </tr>
                </thead>
                <tbody>
                  {visible.map((r) => (
                    <tr key={r.name} className="group border-b border-line last:border-0 hover:bg-surface-hover/50">
                      <td className="px-3 py-2">
                        <input type="checkbox" aria-label={`Select ${r.name}`} className="accent-[var(--accent)]" checked={checked.has(r.name)} onChange={(e) => { const n = new Set(checked); if (e.target.checked) n.add(r.name); else n.delete(r.name); setChecked(n); }} />
                      </td>
                      <td className="selectable max-w-[280px] truncate px-3 py-2 font-mono text-xs text-fg" title={r.name}>{r.name}</td>
                      <td className="selectable max-w-[160px] truncate px-3 py-2 font-mono text-xs text-accent-2">{displayValue(r.value)}</td>
                      <td className="px-3 py-2 text-xs text-fg-2">{r.flagType}</td>
                      <td className="px-3 py-2 text-xs capitalize text-fg-2">{r.category}</td>
                      <td className="px-3 py-2" title={r.messages.join("\n")}>
                        <Badge tone={STATUS_LABEL[r.status].tone}>{STATUS_LABEL[r.status].label}</Badge>
                      </td>
                      <td className="px-2 py-1 text-right">
                        <span className="inline-flex opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                          <IconButton label="Edit" onClick={() => editFlag(r)}><Pencil size={13} /></IconButton>
                          <IconButton label="Delete" onClick={() => pid && run(() => api.ffRemoveFlags(pid, [r.name]))}><Trash2 size={13} /></IconButton>
                        </span>
                      </td>
                    </tr>
                  ))}
                  {visible.length === 0 && (
                    <tr><td colSpan={7} className="px-3 py-8 text-center text-xs text-fg-3">No flags match the filters.</td></tr>
                  )}
                </tbody>
              </table>
            )}
          </Card>

          <div className="mt-3 flex items-center gap-2">
            <span className="text-xs text-fg-3">{rows.data?.length ?? 0} flags · {counts.allowlisted} allowlisted{settings?.advanced.developerMode && pid ? <span className="selectable font-mono"> · {pid}</span> : null}</span>
            <div className="flex-1" />
            <Button size="sm" variant="ghost" icon={<RotateCcw size={13} />} disabled={!pid || !rows.data?.length} onClick={async () => {
              const ok = await dialogs.confirm({ title: `Reset "${profile?.name}"?`, description: "All flags in this profile are removed. A backup is made first and can be restored.", confirmLabel: "Reset", tone: "danger" });
              if (ok && pid) await run(() => api.ffReset(pid), "Profile reset");
            }}>Reset profile</Button>
            <Button size="sm" icon={<Send size={13} />} disabled={!pid} onClick={async () => {
              if (!pid) return;
              try {
                const n = await api.ffApplyNow(pid);
                toast.success("Applied to installation", `${n} flags written to ClientAppSettings.json`);
              } catch (e) { dialogs.showError("Could not apply", e); }
            }}>Apply now</Button>
          </div>

          {presets.data && presets.data.length > 0 && (
            <div className="mt-5">
              <div className="mb-2 flex items-center gap-1.5 text-[11px] font-semibold uppercase tracking-wider text-fg-3"><Blocks size={12} /> Presets from plugins</div>
              <div className="grid grid-cols-3 gap-2">
                {presets.data.map((p) => (
                  <Card key={p.pluginId + p.preset.name} className="p-3">
                    <div className="text-[13px] font-medium text-fg">{p.preset.name}</div>
                    <div className="mt-0.5 text-xs text-fg-2">{p.preset.description}</div>
                    <div className="mt-2 flex items-center justify-between">
                      <span className="text-[11px] text-fg-3">{p.pluginName} · {Object.keys(p.preset.flags).length} flags</span>
                      <Button size="sm" disabled={!pid} onClick={() => pid && run(() => api.ffImport(pid, JSON.stringify(p.preset.flags), false), "Preset added", p.preset.name)}>Add</Button>
                    </div>
                  </Card>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
