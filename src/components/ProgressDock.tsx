import { AnimatePresence, motion } from "framer-motion";
import { CheckCircle2, X, XCircle } from "lucide-react";
import { useState } from "react";
import { api } from "../services/api";
import { useApp } from "../stores/app";
import { formatBytes, formatDuration } from "../utils/format";
import { Button, ProgressBar, Spinner } from "./ui";

/** Shows the currently running long operation with real byte counts. */
export function ProgressView({ compact }: { compact?: boolean }) {
  const { progress: p } = useApp();
  const [cancelling, setCancelling] = useState(false);
  if (!p) return null;
  const ratio = p.total ? p.downloaded / p.total : p.steps ? p.step / p.steps : null;
  const indeterminate = !p.done && (ratio == null || (p.stage !== "Downloading" && !p.steps && !p.total));
  const failed = p.done && (p.stage === "Failed" || p.stage === "Cancelled");
  return (
    <div>
      <div className="flex items-center gap-2.5">
        {p.done ? failed ? <XCircle size={16} className="text-danger" /> : <CheckCircle2 size={16} className="text-success" /> : <Spinner size={15} />}
        <div className="min-w-0 flex-1">
          <div className="truncate text-[13px] font-medium text-fg">{p.operation}</div>
          <div className="truncate text-xs text-fg-2">
            {p.stage}
            {p.detail ? ` · ${p.detail}` : ""}
          </div>
        </div>
        {!p.done && (
          <Button
            size="sm"
            variant="ghost"
            loading={cancelling}
            icon={<X size={13} />}
            onClick={async () => {
              setCancelling(true);
              await api.cancelOperation().catch(() => {});
              window.setTimeout(() => setCancelling(false), 1500);
            }}
          >
            Cancel
          </Button>
        )}
      </div>
      {!p.done && (
        <div className="mt-2.5">
          <ProgressBar value={ratio} indeterminate={indeterminate} />
          {!compact && (
            <div className="mt-1.5 flex justify-between text-[11px] text-fg-3">
              <span>
                {p.total ? `${formatBytes(p.downloaded)} of ${formatBytes(p.total)}` : p.steps ? `${p.step} of ${p.steps}` : ""}
              </span>
              <span>
                {p.bytesPerSecond ? `${formatBytes(p.bytesPerSecond)}/s` : ""}
                {p.etaSeconds ? ` · ${formatDuration(p.etaSeconds)} left` : ""}
              </span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function ProgressDock() {
  const { progress, page } = useApp();
  // Pages with their own progress panel don't need the floating dock.
  const show = !!progress && page !== "installations" && page !== "home";
  return (
    <AnimatePresence>
      {show && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 10 }}
          className="fixed bottom-4 left-1/2 z-40 w-[420px] -translate-x-1/2 rounded-xl border border-line bg-surface p-3.5 shadow-[var(--shadow)]"
        >
          <ProgressView />
        </motion.div>
      )}
    </AnimatePresence>
  );
}
