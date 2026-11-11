import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, CheckCircle2, Info, X, XCircle } from "lucide-react";

export type ToastLevel = "success" | "info" | "warning" | "error";
export interface Toast {
  id: number;
  level: ToastLevel;
  title: string;
  description?: string;
  action?: { label: string; onClick: () => void };
}

interface ToastApi {
  push: (t: Omit<Toast, "id">) => void;
  success: (title: string, description?: string) => void;
  info: (title: string, description?: string) => void;
  warning: (title: string, description?: string) => void;
  error: (title: string, description?: string, action?: Toast["action"]) => void;
}

const Ctx = createContext<ToastApi | null>(null);

const ICONS = { success: CheckCircle2, info: Info, warning: AlertTriangle, error: XCircle };
const COLORS = { success: "text-success", info: "text-accent", warning: "text-warning", error: "text-danger" };

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const next = useRef(1);
  const dismiss = useCallback((id: number) => setToasts((t) => t.filter((x) => x.id !== id)), []);
  const push = useCallback(
    (t: Omit<Toast, "id">) => {
      const id = next.current++;
      setToasts((list) => [...list.slice(-4), { ...t, id }]);
      // Errors stay longer; they carry information the user may need to read.
      const ms = t.level === "error" ? 9000 : t.level === "warning" ? 7000 : 4500;
      window.setTimeout(() => dismiss(id), ms);
    },
    [dismiss],
  );
  const api = useMemo<ToastApi>(
    () => ({
      push,
      success: (title, description) => push({ level: "success", title, description }),
      info: (title, description) => push({ level: "info", title, description }),
      warning: (title, description) => push({ level: "warning", title, description }),
      error: (title, description, action) => push({ level: "error", title, description, action }),
    }),
    [push],
  );
  return (
    <Ctx.Provider value={api}>
      {children}
      <div className="pointer-events-none fixed bottom-4 right-4 z-[60] flex w-[360px] flex-col gap-2" role="status" aria-live="polite">
        <AnimatePresence initial={false}>
          {toasts.map((t) => {
            const Icon = ICONS[t.level];
            return (
              <motion.div
                key={t.id}
                layout
                initial={{ opacity: 0, y: 12, scale: 0.98 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{ opacity: 0, x: 24 }}
                transition={{ duration: 0.18 }}
                className="pointer-events-auto flex gap-3 rounded-xl border border-line bg-surface p-3 shadow-[var(--shadow)]"
              >
                <Icon size={18} className={`mt-0.5 shrink-0 ${COLORS[t.level]}`} />
                <div className="min-w-0 flex-1">
                  <div className="text-[13px] font-semibold text-fg">{t.title}</div>
                  {t.description && <div className="selectable mt-0.5 break-words text-xs text-fg-2">{t.description}</div>}
                  {t.action && (
                    <button
                      className="mt-1.5 text-xs font-medium text-accent hover:underline"
                      onClick={() => {
                        t.action!.onClick();
                        dismiss(t.id);
                      }}
                    >
                      {t.action.label}
                    </button>
                  )}
                </div>
                <button aria-label="Dismiss" className="h-6 w-6 shrink-0 rounded-md text-fg-3 hover:bg-surface-hover hover:text-fg" onClick={() => dismiss(t.id)}>
                  <X size={14} className="mx-auto" />
                </button>
              </motion.div>
            );
          })}
        </AnimatePresence>
      </div>
    </Ctx.Provider>
  );
}

export function useToast(): ToastApi {
  const c = useContext(Ctx);
  if (!c) throw new Error("useToast outside ToastProvider");
  return c;
}
