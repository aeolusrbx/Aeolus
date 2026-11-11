import { useEffect, useState } from "react";
import { ArchiveRestore } from "lucide-react";
import { Modal, useDialogs } from "../stores/dialogs";
import { api, errorMessage } from "../services/api";
import type { BackupEntry } from "../types";
import { formatBytes, formatDate } from "../utils/format";
import { Badge, Button, EmptyState, Spinner } from "./ui";
import { useToast } from "../stores/toasts";

const LABELS: Record<string, string> = {
  fastflags: "Fast Flags (auto)",
  "config-fastflags": "Fast Flags",
  settings: "Settings (auto)",
  "config-settings": "Settings",
  "config-profiles": "Profiles",
};

function RestoreView({ close }: { close: () => void }) {
  const [list, setList] = useState<BackupEntry[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const dialogs = useDialogs();
  const toast = useToast();
  const load = () => api.backupsOverview().then(setList).catch((e) => setErr(errorMessage(e)));
  useEffect(() => {
    void load();
  }, []);
  const restore = async (b: BackupEntry) => {
    const ok = await dialogs.confirm({
      title: "Restore this backup?",
      description: `${LABELS[b.category] ?? b.category} from ${formatDate(b.createdAt)} will replace the current version. The current version is backed up first, so this can be undone.`,
      confirmLabel: "Restore",
      tone: "warning",
    });
    if (!ok) return;
    setBusy(b.id);
    try {
      await api.restoreBackup(b.category, b.id);
      toast.success("Configuration restored", LABELS[b.category] ?? b.category);
      await load();
    } catch (e) {
      dialogs.showError("Restore failed", e);
    } finally {
      setBusy(null);
    }
  };
  return (
    <Modal onClose={close} wide>
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-[15px] font-semibold text-fg">Restore configuration</h3>
          <p className="mt-1 text-[13px] text-fg-2">Backups are created automatically before changes and destructive operations.</p>
        </div>
        <Button
          size="sm"
          onClick={async () => {
            try {
              const n = await api.createManualBackup();
              toast.success("Backup created", `${n} configuration files saved`);
              await load();
            } catch (e) {
              dialogs.showError("Backup failed", e);
            }
          }}
        >
          Back up now
        </Button>
      </div>
      <div className="mt-4 max-h-[420px] overflow-auto rounded-lg border border-line">
        {err && <div className="p-4 text-[13px] text-danger">{err}</div>}
        {!list && !err && (
          <div className="flex justify-center p-8"><Spinner /></div>
        )}
        {list && list.length === 0 && <EmptyState icon={<ArchiveRestore size={22} />} title="No backups yet" description="Backups appear here after you change settings, profiles or Fast Flags." />}
        {list?.map((b) => (
          <div key={b.category + b.id} className="flex items-center gap-3 border-b border-line px-3.5 py-2.5 last:border-0">
            <Badge tone="neutral">{LABELS[b.category] ?? b.category}</Badge>
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13px] text-fg">{b.label.replace(/-/g, " ") || "backup"}</div>
              <div className="text-[11px] text-fg-3">{formatDate(b.createdAt)} · {formatBytes(b.size)}</div>
            </div>
            <Button size="sm" loading={busy === b.id} onClick={() => restore(b)}>Restore</Button>
          </div>
        ))}
      </div>
      <div className="mt-4 flex justify-end">
        <Button variant="primary" onClick={close}>Done</Button>
      </div>
    </Modal>
  );
}

export function useRestoreDialog() {
  const dialogs = useDialogs();
  return () => dialogs.custom<void>((close) => <RestoreView close={() => close(null)} />);
}
