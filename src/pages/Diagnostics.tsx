import { useMemo, useState } from "react";
import { save as saveDialog } from "@tauri-apps/plugin-dialog";
import { Cpu, Download, FileText, MemoryStick, Monitor, RefreshCw, Search, Trash2 } from "lucide-react";
import { api } from "../services/api";
import { useApp } from "../stores/app";
import { useResource } from "../hooks/useResource";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import { Badge, Button, Card, EmptyState, Input, PageHeader, SectionTitle, Spinner, cx } from "../components/ui";
import type { LogLevel, LogQuery } from "../types";
import { formatBytes } from "../utils/format";

const LEVELS: LogLevel[] = ["ERROR", "WARNING", "INFO", "DEBUG"];
const LEVEL_TONE: Record<LogLevel, string> = { ERROR: "text-danger", WARNING: "text-warning", INFO: "text-fg-2", DEBUG: "text-fg-3" };

function displayInfo() {
  const r = window.devicePixelRatio || 1;
  return `${Math.round(window.screen.width * r)}×${Math.round(window.screen.height * r)} (scale ${Math.round(r * 100)}%)`;
}

export default function Diagnostics() {
  const { info } = useApp();
  const sys = useResource(api.diagnosticsSystem, []);
  const ov = useResource(api.installOverview, ["install"]);
  const dialogs = useDialogs();
  const toast = useToast();
  const [search, setSearch] = useState("");
  const [levels, setLevels] = useState<LogLevel[]>(["ERROR", "WARNING", "INFO"]);
  const [from, setFrom] = useState("");
  const [to, setTo] = useState("");
  const query: LogQuery = useMemo(
    () => ({
      search: search || undefined,
      levels,
      from: from ? new Date(from + "T00:00:00").toISOString() : undefined,
      to: to ? new Date(to + "T23:59:59").toISOString() : undefined,
      limit: 1000,
    }),
    [search, levels, from, to],
  );
  const logs = useResource(() => api.logsQuery(query), ["install", "launch", "settings", "fastflags", "plugins", "profiles", "all"], [query]);

  const s = sys.data;
  const active = ov.data?.state.active;

  const rows: [string, string][] = [
    ["Windows", s ? s.osName : "…"],
    ["Build", s ? `${s.osVersion} (build ${s.osBuild})` : "…"],
    ["Architecture", s?.architecture ?? "…"],
    ["CPU", s ? `${s.cpu} · ${s.cpuCores} threads` : "…"],
    ["GPU", s ? (s.gpus.length ? s.gpus.join(", ") : "Not detected") : "…"],
    ["Installed RAM", s ? formatBytes(s.totalMemory) : "…"],
    ["Available RAM", s ? formatBytes(s.availableMemory) : "…"],
    ["Display", displayInfo()],
    ["Aeolus", info?.version ?? "…"],
    ["Roblox", active ? `${active.version} (${active.versionGuid})` : ov.data?.state.preferredExternal ? "External installation" : "Not installed"],
    ["Install folder", active?.path ?? ov.data?.installRoot ?? "…"],
  ];

  return (
    <div className="mx-auto flex h-full max-w-5xl flex-col px-8 py-8">
      <PageHeader
        title="Diagnostics"
        description="System details, logs and exportable reports for troubleshooting. No passwords, cookies or tokens are ever collected."
        actions={
          <Button
            variant="primary"
            icon={<FileText size={15} />}
            onClick={async () => {
              const f = await saveDialog({ defaultPath: `aeolus-diagnostics-${new Date().toISOString().slice(0, 10)}.json`, filters: [{ name: "JSON", extensions: ["json"] }] }).catch(() => null);
              if (!f) return;
              try {
                await api.diagnosticsReport(f, displayInfo());
                toast.success("Diagnostic report saved", f);
              } catch (e) {
                dialogs.showError("Could not create report", e);
              }
            }}
          >
            Export report
          </Button>
        }
      />

      <SectionTitle action={<Button size="sm" variant="ghost" icon={<RefreshCw size={12} />} onClick={() => sys.reload()}>Refresh</Button>}>System</SectionTitle>
      <Card className="mb-6 grid grid-cols-2 gap-x-8 px-5 py-3">
        {rows.map(([k, v]) => (
          <div key={k} className="flex gap-3 border-b border-line py-2 text-[13px] last:border-0 [&:nth-last-child(2)]:border-0">
            <span className="w-28 shrink-0 text-fg-3">{k}</span>
            <span className="selectable min-w-0 break-words text-fg">{v}</span>
          </div>
        ))}
      </Card>
      {s && (
        <div className="-mt-3 mb-6 flex gap-4 text-[11px] text-fg-3">
          <span className="inline-flex items-center gap-1"><Cpu size={11} />CPU</span>
          <span className="inline-flex items-center gap-1"><MemoryStick size={11} />{Math.round(((s.totalMemory - s.availableMemory) / s.totalMemory) * 100)}% memory in use</span>
          <span className="inline-flex items-center gap-1"><Monitor size={11} />{displayInfo()}</span>
        </div>
      )}

      <SectionTitle
        action={
          <div className="flex gap-1">
            <Button
              size="sm"
              variant="ghost"
              icon={<Download size={12} />}
              onClick={async () => {
                const f = await saveDialog({ defaultPath: "aeolus-logs.txt", filters: [{ name: "Text", extensions: ["txt", "log"] }] }).catch(() => null);
                if (f) api.logsExport(query, f).then((n) => toast.success(`Exported ${n} log entries`)).catch((e) => dialogs.showError("Export failed", e));
              }}
            >
              Export
            </Button>
            <Button
              size="sm"
              variant="ghost"
              icon={<Trash2 size={12} />}
              onClick={async () => {
                const ok = await dialogs.confirm({ title: "Clear all logs?", description: "All log files are deleted. This cannot be undone.", confirmLabel: "Clear logs", tone: "danger" });
                if (ok) api.logsClear().then(() => { toast.success("Logs cleared"); logs.reload(); }).catch((e) => dialogs.showError("Could not clear logs", e));
              }}
            >
              Clear
            </Button>
          </div>
        }
      >
        Logs
      </SectionTitle>
      <div className="mb-2 flex flex-wrap items-center gap-2">
        <div className="relative w-64">
          <Search size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-fg-3" />
          <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search logs" className="pl-8" aria-label="Search logs" />
        </div>
        {LEVELS.map((l) => (
          <button
            key={l}
            onClick={() => setLevels((ls) => (ls.includes(l) ? ls.filter((x) => x !== l) : [...ls, l]))}
            className={cx("h-7 rounded-md border px-2.5 text-[11px] font-semibold tracking-wide transition-colors", levels.includes(l) ? "border-accent/50 bg-accent/10 text-fg" : "border-line text-fg-3 hover:text-fg")}
          >
            {l}
          </button>
        ))}
        <div className="ml-auto flex items-center gap-1.5 text-xs text-fg-3">
          <Input type="date" value={from} onChange={(e) => setFrom(e.target.value)} className="h-8 w-36" aria-label="From date" />
          –
          <Input type="date" value={to} onChange={(e) => setTo(e.target.value)} className="h-8 w-36" aria-label="To date" />
        </div>
      </div>
      <Card className="min-h-[260px] flex-1 overflow-auto font-mono text-[11.5px]">
        {logs.loading && !logs.data && <div className="flex justify-center p-8"><Spinner /></div>}
        {logs.data?.length === 0 && <EmptyState title="No log entries match" />}
        {logs.data?.map((e, i) => (
          <div key={i} className="selectable flex gap-3 border-b border-line/60 px-3 py-1.5 last:border-0 hover:bg-surface-hover/40">
            <span className="shrink-0 text-fg-3">{new Date(e.ts).toLocaleString()}</span>
            <span className={cx("w-[4.5rem] shrink-0 font-semibold", LEVEL_TONE[e.level])}>{e.level}</span>
            <span className="w-24 shrink-0 truncate text-accent-2" title={e.source}>{e.source}</span>
            <span className="min-w-0 flex-1 break-words text-fg">{e.message}</span>
          </div>
        ))}
      </Card>
      {logs.data && logs.data.length >= 1000 && <div className="mt-1.5 text-[11px] text-fg-3"><Badge>Showing newest 1000</Badge></div>}
    </div>
  );
}
