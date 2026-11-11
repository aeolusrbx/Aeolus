import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { AlertTriangle, Info, Trash2, XCircle } from "lucide-react";
import { Button, Input } from "../components/ui";
import { errorHint, errorMessage } from "../services/api";

type Tone = "default" | "danger" | "warning";

export interface ConfirmOptions {
  title: string;
  description?: ReactNode;
  confirmLabel?: string;
  cancelLabel?: string;
  tone?: Tone;
  /** Optional checkbox (e.g. "Also clear download cache"). */
  checkbox?: { label: string; defaultChecked?: boolean };
  /** Require typing this text to enable the confirm button. */
  typeToConfirm?: string;
}
export interface ConfirmResult {
  checked: boolean;
}
export interface PromptOptions {
  title: string;
  description?: ReactNode;
  label?: string;
  initial?: string;
  placeholder?: string;
  confirmLabel?: string;
  validate?: (v: string) => string | null;
  multiline?: boolean;
}

interface DialogApi {
  confirm: (o: ConfirmOptions) => Promise<ConfirmResult | null>;
  prompt: (o: PromptOptions) => Promise<string | null>;
  showError: (title: string, error: unknown) => void;
  custom: <T>(render: (close: (v: T | null) => void) => ReactNode) => Promise<T | null>;
}

const Ctx = createContext<DialogApi | null>(null);

interface Entry {
  id: number;
  node: (close: (v: unknown) => void) => ReactNode;
  resolve: (v: unknown) => void;
}

export function Modal({ children, onClose, wide }: { children: ReactNode; onClose: () => void; wide?: boolean }) {
  useEffect(() => {
    const h = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, [onClose]);
  return (
    <motion.div className="fixed inset-0 z-50 flex items-center justify-center p-6" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }}>
      <div className="absolute inset-0 bg-black/55" onClick={onClose} />
      <motion.div
        role="dialog"
        aria-modal="true"
        initial={{ opacity: 0, y: 8, scale: 0.98 }}
        animate={{ opacity: 1, y: 0, scale: 1 }}
        exit={{ opacity: 0, y: 8, scale: 0.98 }}
        transition={{ duration: 0.16 }}
        className={`relative max-h-full w-full overflow-auto rounded-2xl border border-line bg-surface p-5 shadow-[var(--shadow)] ${wide ? "max-w-2xl" : "max-w-md"}`}
      >
        {children}
      </motion.div>
    </motion.div>
  );
}

function ToneIcon({ tone }: { tone: Tone }) {
  const cls = "flex h-9 w-9 shrink-0 items-center justify-center rounded-full";
  if (tone === "danger") return <div className={`${cls} bg-danger/12 text-danger`}><Trash2 size={17} /></div>;
  if (tone === "warning") return <div className={`${cls} bg-warning/12 text-warning`}><AlertTriangle size={17} /></div>;
  return <div className={`${cls} bg-accent/12 text-accent`}><Info size={17} /></div>;
}

function ConfirmView({ o, close }: { o: ConfirmOptions; close: (v: ConfirmResult | null) => void }) {
  const [checked, setChecked] = useState(!!o.checkbox?.defaultChecked);
  const [typed, setTyped] = useState("");
  const tone = o.tone ?? "default";
  const blocked = !!o.typeToConfirm && typed !== o.typeToConfirm;
  return (
    <Modal onClose={() => close(null)}>
      <div className="flex gap-3.5">
        <ToneIcon tone={tone} />
        <div className="min-w-0 flex-1">
          <h3 className="text-[15px] font-semibold text-fg">{o.title}</h3>
          {o.description && <div className="mt-1.5 text-[13px] leading-relaxed text-fg-2">{o.description}</div>}
          {o.checkbox && (
            <label className="mt-3 flex cursor-pointer items-center gap-2 text-[13px] text-fg-2">
              <input type="checkbox" className="accent-[var(--accent)]" checked={checked} onChange={(e) => setChecked(e.target.checked)} />
              {o.checkbox.label}
            </label>
          )}
          {o.typeToConfirm && (
            <div className="mt-3">
              <div className="mb-1 text-xs text-fg-2">Type <span className="font-mono text-fg">{o.typeToConfirm}</span> to confirm</div>
              <Input autoFocus value={typed} onChange={(e) => setTyped(e.target.value)} />
            </div>
          )}
        </div>
      </div>
      <div className="mt-5 flex justify-end gap-2">
        <Button variant="ghost" onClick={() => close(null)}>{o.cancelLabel ?? "Cancel"}</Button>
        <Button variant={tone === "danger" ? "danger" : "primary"} disabled={blocked} autoFocus={!o.typeToConfirm} onClick={() => close({ checked })}>
          {o.confirmLabel ?? "Confirm"}
        </Button>
      </div>
    </Modal>
  );
}

function PromptView({ o, close }: { o: PromptOptions; close: (v: string | null) => void }) {
  const [v, setV] = useState(o.initial ?? "");
  const err = o.validate?.(v) ?? null;
  const submit = () => !err && close(v);
  return (
    <Modal onClose={() => close(null)} wide={o.multiline}>
      <h3 className="text-[15px] font-semibold text-fg">{o.title}</h3>
      {o.description && <div className="mt-1.5 text-[13px] text-fg-2">{o.description}</div>}
      <div className="mt-4">
        {o.label && <label className="mb-1.5 block text-xs font-medium text-fg-2">{o.label}</label>}
        {o.multiline ? (
          <textarea
            autoFocus
            rows={10}
            className="w-full rounded-lg border border-line bg-bg-2 p-3 font-mono text-xs text-fg focus:border-accent/60 focus:outline-none"
            placeholder={o.placeholder}
            value={v}
            onChange={(e) => setV(e.target.value)}
          />
        ) : (
          <Input autoFocus placeholder={o.placeholder} value={v} onChange={(e) => setV(e.target.value)} onKeyDown={(e) => e.key === "Enter" && submit()} />
        )}
        {err && v.length > 0 && <div className="mt-1.5 text-xs text-danger">{err}</div>}
      </div>
      <div className="mt-5 flex justify-end gap-2">
        <Button variant="ghost" onClick={() => close(null)}>Cancel</Button>
        <Button variant="primary" disabled={!!err} onClick={submit}>{o.confirmLabel ?? "Save"}</Button>
      </div>
    </Modal>
  );
}

function ErrorView({ title, error, close }: { title: string; error: unknown; close: () => void }) {
  const hint = errorHint(error);
  return (
    <Modal onClose={close}>
      <div className="flex gap-3.5">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-danger/12 text-danger"><XCircle size={17} /></div>
        <div className="min-w-0 flex-1">
          <h3 className="text-[15px] font-semibold text-fg">{title}</h3>
          <div className="selectable mt-1.5 break-words text-[13px] text-fg-2">{errorMessage(error)}</div>
          {hint && <div className="mt-3 rounded-lg border border-line bg-bg-2 p-3 text-xs text-fg-2">{hint}</div>}
        </div>
      </div>
      <div className="mt-5 flex justify-end">
        <Button variant="primary" autoFocus onClick={close}>Close</Button>
      </div>
    </Modal>
  );
}

export function DialogProvider({ children }: { children: ReactNode }) {
  const [stack, setStack] = useState<Entry[]>([]);
  const next = useRef(1);
  const open = useCallback(<T,>(node: (close: (v: T | null) => void) => ReactNode) => {
    return new Promise<T | null>((resolve) => {
      const id = next.current++;
      setStack((s) => [...s, { id, node: node as Entry["node"], resolve: resolve as Entry["resolve"] }]);
    });
  }, []);
  const closeEntry = (e: Entry, v: unknown) => {
    setStack((s) => s.filter((x) => x.id !== e.id));
    e.resolve(v);
  };
  const api: DialogApi = {
    confirm: (o) => open<ConfirmResult>((close) => <ConfirmView o={o} close={close} />),
    prompt: (o) => open<string>((close) => <PromptView o={o} close={close} />),
    showError: (title, error) => void open<void>((close) => <ErrorView title={title} error={error} close={() => close(null)} />),
    custom: open,
  };
  return (
    <Ctx.Provider value={api}>
      {children}
      <AnimatePresence>
        {stack.map((e) => (
          <div key={e.id}>{e.node((v) => closeEntry(e, v))}</div>
        ))}
      </AnimatePresence>
    </Ctx.Provider>
  );
}

export function useDialogs(): DialogApi {
  const c = useContext(Ctx);
  if (!c) throw new Error("useDialogs outside DialogProvider");
  return c;
}
