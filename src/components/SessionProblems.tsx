import { AlertTriangle, RotateCw, ShieldCheck, WifiOff, X } from "lucide-react";
import { useEffect, useState } from "react";
import { api, errorMessage, isCancelled } from "../services/api";
import { useApp } from "../stores/app";
import { useToast } from "../stores/toasts";
import type { CrashReport, Disconnect } from "../types";
import { relativeTime } from "../utils/format";
import { GameIcon, gameName } from "./GameHistory";
import { Badge, Button, Card } from "./ui";

function useSecondsUntil(at: string | null) {
  const calc = () => (at ? Math.max(0, Math.ceil((new Date(at).getTime() - Date.now()) / 1000)) : null);
  const [s, setS] = useState(calc);
  useEffect(() => {
    setS(calc());
    if (!at) return;
    const t = setInterval(() => setS(calc()), 250);
    return () => clearInterval(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [at]);
  return s;
}

export function DisconnectCard({ d, onChanged }: { d: Disconnect; onChanged: () => void }) {
  const toast = useToast();
  const { progress } = useApp();
  const [busy, setBusy] = useState(false);
  const secs = useSecondsUntil(d.autoAt);
  const rejoin = async () => {
    setBusy(true);
    try {
      await api.disconnectRejoin();
      toast.success(`Rejoining ${gameName(d.game)}`);
    } catch (e) {
      if (!isCancelled(e)) toast.error("Couldn't rejoin", errorMessage(e));
    } finally {
      setBusy(false);
      onChanged();
    }
  };
  const dismiss = () => api.disconnectDismiss().then(onChanged);
  return (
    <Card className="mt-4 border-warning/40 p-4" role="alert">
      <div className="flex items-start gap-3">
        <GameIcon game={d.game} size={44} />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <WifiOff size={14} className="text-warning" />
            <span className="text-[13px] font-semibold text-fg">Disconnected from {gameName(d.game)}</span>
            {d.code !== null && <Badge tone="warning">Error {d.code}</Badge>}
            <span className="text-xs text-fg-3">{relativeTime(d.at)}</span>
          </div>
          <div className="mt-1 text-[13px] text-fg">{d.title}</div>
          <div className="text-xs text-fg-2">{d.detail}</div>
          {d.canRejoin && !d.sameServer && <div className="mt-1 text-[11px] text-fg-3">This was a private server, so Aeolus joins the game in a new server.</div>}
          {secs !== null && d.canRejoin && <div className="mt-2 text-xs font-medium text-accent">Rejoining automatically in {secs}s…</div>}
        </div>
        <button aria-label="Dismiss" title={secs !== null ? "Cancel" : "Dismiss"} className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-fg-3 hover:bg-surface-hover hover:text-fg" onClick={dismiss}>
          <X size={15} />
        </button>
      </div>
      {d.canRejoin && (
        <div className="mt-3 flex justify-end gap-2">
          {secs !== null && <Button size="sm" variant="ghost" onClick={dismiss}>Cancel</Button>}
          <Button size="sm" variant="primary" icon={<RotateCw size={13} />} loading={busy} disabled={!!progress && !progress.done} onClick={rejoin}>
            {secs !== null ? "Rejoin now" : "Rejoin"}
          </Button>
        </div>
      )}
    </Card>
  );
}

export function CrashCard({ c, onChanged }: { c: CrashReport; onChanged: () => void }) {
  const { navigate } = useApp();
  return (
    <Card className="mt-4 border-danger/40 p-4" role="alert">
      <div className="flex items-start gap-3">
        <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-lg bg-danger/10 text-danger">
          <AlertTriangle size={20} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-[13px] font-semibold text-fg">{c.title}</span>
            <Badge tone="danger">{c.exitCode}</Badge>
            <span className="text-xs text-fg-3">{relativeTime(c.at)}{c.game ? ` · in ${c.game}` : ""}</span>
          </div>
          <div className="mt-1 text-xs text-fg-2">{c.explanation}</div>
          <div className="mt-2 text-xs font-medium text-fg">What to try</div>
          <ul className="mt-1 list-disc space-y-0.5 pl-4 text-xs text-fg-2">
            {c.suggestions.map((s) => <li key={s}>{s}</li>)}
          </ul>
        </div>
        <button aria-label="Dismiss" title="Dismiss" className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md text-fg-3 hover:bg-surface-hover hover:text-fg" onClick={() => api.crashDismiss().then(onChanged)}>
          <X size={15} />
        </button>
      </div>
      <div className="mt-3 flex justify-end gap-2">
        <Button size="sm" variant="ghost" onClick={() => navigate("diagnostics")}>Open Diagnostics</Button>
        <Button size="sm" icon={<ShieldCheck size={13} />} onClick={() => navigate("installations")}>Verify files</Button>
      </div>
    </Card>
  );
}
