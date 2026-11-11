import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, Blocks, ChevronDown, FolderInput, Lock, ShieldCheck, Store, Trash2 } from "lucide-react";
import { api } from "../services/api";
import { useResource } from "../hooks/useResource";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import { Badge, Button, Card, EmptyState, Input, PageHeader, SectionTitle, Spinner, Toggle, cx } from "../components/ui";
import type { PluginInfo } from "../types";
import { formatDate } from "../utils/format";

const STATUS: Record<PluginInfo["status"], { label: string; tone: "success" | "neutral" | "warning" | "danger" | "accent" }> = {
  enabled: { label: "Enabled", tone: "success" },
  disabled: { label: "Disabled", tone: "neutral" },
  "needs-approval": { label: "Needs approval", tone: "accent" },
  changed: { label: "Changed — re-approve", tone: "warning" },
  error: { label: "Error", tone: "danger" },
};

function ConfigEditor({ p }: { p: PluginInfo }) {
  const toast = useToast();
  const dialogs = useDialogs();
  const fields = p.manifest?.contributes.configuration ?? [];
  if (fields.length === 0) return <div className="text-xs text-fg-3">This plugin has no settings.</div>;
  const save = (key: string, value: unknown) => api.pluginSetConfig(p.id, key, value).then(() => toast.success("Plugin setting saved")).catch((e) => dialogs.showError("Invalid value", e));
  return (
    <div className="space-y-3">
      {fields.map((f) => (
        <div key={f.key} className="flex items-center justify-between gap-4">
          <div>
            <div className="text-[13px] text-fg">{f.label}</div>
            {f.description && <div className="text-xs text-fg-3">{f.description}</div>}
          </div>
          {f.type === "boolean" ? (
            <Toggle checked={!!p.config[f.key]} onChange={(v) => save(f.key, v)} />
          ) : (
            <Input
              className="w-64"
              defaultValue={String(p.config[f.key] ?? "")}
              onBlur={(e) => {
                const raw = e.target.value;
                if (raw === String(p.config[f.key] ?? "")) return;
                save(f.key, f.type === "integer" ? (/^-?\d+$/.test(raw.trim()) ? Number(raw) : raw) : raw);
              }}
            />
          )}
        </div>
      ))}
    </div>
  );
}

export default function Plugins() {
  const list = useResource(api.pluginsList, ["plugins"]);
  const dialogs = useDialogs();
  const toast = useToast();
  const [open, setOpen] = useState<string | null>(null);

  const enable = async (p: PluginInfo) => {
    const m = p.manifest!;
    const ok = await dialogs.confirm({
      title: `Enable ${m.name}?`,
      description: (
        <div>
          <div>{m.name} {m.version} by {m.author || "unknown author"} asks for:</div>
          <ul className="mt-2 space-y-1">
            {p.permissions.map(([perm, desc]) => (
              <li key={perm} className="flex items-center gap-2 text-fg"><Lock size={12} className="text-accent" /> {desc} <span className="font-mono text-[11px] text-fg-3">{perm}</span></li>
            ))}
            {p.permissions.length === 0 && <li className="text-fg-3">No permissions</li>}
          </ul>
          <div className="mt-3 text-xs text-fg-3">Plugins are declarative and cannot run code. If the plugin's files change later, Aeolus disables it until you approve it again.</div>
        </div>
      ),
      confirmLabel: "Approve & enable",
    });
    if (!ok) return;
    try {
      await api.pluginEnable(p.id);
      toast.success(`${m.name} enabled`);
    } catch (e) {
      dialogs.showError("Could not enable plugin", e);
    }
  };

  return (
    <div className="mx-auto max-w-4xl px-8 py-8">
      <PageHeader
        title="Plugins"
        description="Extend Aeolus with declarative plugins. Every plugin must declare its permissions and be approved before it does anything."
        actions={
          <Button
            variant="primary"
            icon={<FolderInput size={15} />}
            onClick={async () => {
              const dir = await openDialog({ directory: true, title: "Select a plugin folder (containing aeolus-plugin.json)" }).catch(() => null);
              if (typeof dir !== "string") return;
              try {
                const name = await api.pluginInstall(dir);
                toast.success(`${name} installed`, "Review its permissions to enable it.");
              } catch (e) {
                dialogs.showError("Could not install plugin", e);
              }
            }}
          >
            Install from folder
          </Button>
        }
      />

      {list.loading && !list.data && <div className="flex justify-center py-10"><Spinner /></div>}
      {list.data?.length === 0 && <Card><EmptyState icon={<Blocks size={22} />} title="No plugins installed" description="Install a plugin from a folder containing an aeolus-plugin.json manifest." /></Card>}

      <div className="space-y-3">
        {list.data?.map((p) => {
          const m = p.manifest;
          const st = STATUS[p.status];
          const expanded = open === p.id;
          return (
            <Card key={p.id} className="overflow-hidden">
              <div className="flex items-start gap-4 p-4">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-accent/12 text-accent"><Blocks size={18} /></div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-[14px] font-semibold text-fg">{m?.name ?? p.id}</span>
                    {m && <span className="text-xs text-fg-3">v{m.version}</span>}
                    <Badge tone={st.tone}>{st.label}</Badge>
                    {!p.dependenciesOk && <Badge tone="warning"><AlertTriangle size={10} /> Dependencies</Badge>}
                  </div>
                  <div className="mt-0.5 text-[13px] text-fg-2">{m?.description ?? p.error}</div>
                  <div className="mt-1.5 text-[11px] text-fg-3">
                    {m?.author ? `By ${m.author} · ` : ""}Updated {formatDate(p.lastUpdated)} · {p.permissions.length} permission{p.permissions.length === 1 ? "" : "s"}
                  </div>
                  {p.error && p.status !== "error" && <div className="mt-1 text-xs text-danger">Last error: {p.error}</div>}
                </div>
                <div className="flex shrink-0 items-center gap-2">
                  {m && (p.status === "enabled" ? (
                    <Button size="sm" onClick={() => api.pluginDisable(p.id).then(() => toast.info(`${m.name} disabled`)).catch((e) => dialogs.showError("Could not disable", e))}>Disable</Button>
                  ) : (
                    <Button size="sm" variant="primary" icon={<ShieldCheck size={13} />} onClick={() => enable(p)}>
                      {p.status === "disabled" ? "Enable" : "Review & enable"}
                    </Button>
                  ))}
                  <button aria-label="Details" className="flex h-7 w-7 items-center justify-center rounded-md text-fg-3 hover:bg-surface-hover hover:text-fg" onClick={() => setOpen(expanded ? null : p.id)}>
                    <ChevronDown size={15} className={cx("transition-transform", expanded && "rotate-180")} />
                  </button>
                </div>
              </div>
              {expanded && (
                <div className="grid grid-cols-2 gap-6 border-t border-line bg-bg-2/50 p-4">
                  <div>
                    <SectionTitle>Details</SectionTitle>
                    <dl className="space-y-1.5 text-xs">
                      <div className="flex gap-2"><dt className="w-24 text-fg-3">Identifier</dt><dd className="selectable font-mono text-fg-2">{p.id}</dd></div>
                      <div className="flex gap-2"><dt className="w-24 text-fg-3">Plugin API</dt><dd className="text-fg-2">v{m?.aeolusApi ?? "?"}</dd></div>
                      <div className="flex gap-2"><dt className="w-24 text-fg-3">Location</dt><dd className="selectable break-all text-fg-2">{p.path}</dd></div>
                      <div className="flex gap-2"><dt className="w-24 text-fg-3">Dependencies</dt><dd className="text-fg-2">{m?.dependencies.length ? m.dependencies.map((d) => `${d.id}${d.minVersion ? ` ≥ ${d.minVersion}` : ""}`).join(", ") : "None"}</dd></div>
                      {p.dependencyIssues.map((d) => <div key={d} className="text-warning">{d}</div>)}
                      <div className="flex gap-2"><dt className="w-24 text-fg-3">Contributes</dt><dd className="text-fg-2">{m ? `${m.contributes.fastFlagPresets.length} Fast Flag presets · ${Object.values(m.contributes.hooks).reduce((n, a) => n + (a?.length ?? 0), 0)} hook actions` : "—"}</dd></div>
                    </dl>
                    <SectionTitle>Permissions</SectionTitle>
                    <ul className="space-y-1 text-xs text-fg-2">
                      {p.permissions.map(([perm, desc]) => <li key={perm} className="flex items-center gap-2"><Lock size={11} className="text-fg-3" />{desc}</li>)}
                    </ul>
                  </div>
                  <div>
                    <SectionTitle>Configuration</SectionTitle>
                    <ConfigEditor p={p} />
                    <div className="mt-6 flex justify-end">
                      <Button
                        size="sm"
                        variant="danger"
                        icon={<Trash2 size={13} />}
                        onClick={async () => {
                          const ok = await dialogs.confirm({ title: `Uninstall ${m?.name ?? p.id}?`, description: "The plugin folder and its settings are removed.", confirmLabel: "Uninstall", tone: "danger" });
                          if (ok) api.pluginUninstall(p.id).then(() => toast.success("Plugin uninstalled")).catch((e) => dialogs.showError("Could not uninstall", e));
                        }}
                      >
                        Uninstall
                      </Button>
                    </div>
                  </div>
                </div>
              )}
            </Card>
          );
        })}
      </div>

      <Card className="mt-8 flex items-start gap-3 p-4">
        <Store size={18} className="mt-0.5 shrink-0 text-fg-3" />
        <div className="text-xs text-fg-2">
          <div className="text-[13px] font-medium text-fg">Plugin marketplace</div>
          There is no plugin marketplace yet. The manifest format already carries what one needs (id, version, API compatibility, dependencies, permissions), and the developer guide in <span className="font-mono">documentation/PLUGINS.md</span> describes how to write a plugin.
        </div>
      </Card>
    </div>
  );
}
