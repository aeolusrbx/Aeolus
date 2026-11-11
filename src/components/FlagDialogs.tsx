import { useEffect, useState } from "react";
import { Modal } from "../stores/dialogs";
import { api, errorMessage } from "../services/api";
import type { Comparison, FlagProfileSummary } from "../types";
import { displayValue, flagTypeOf, parseFlagInput } from "../utils/format";
import { Button, Input, Select, Spinner, Textarea } from "./ui";

export function FlagEditor({
  initialName,
  initialValue,
  allowlist,
  onSave,
  onClose,
}: {
  initialName?: string;
  initialValue?: unknown;
  allowlist: string[];
  onSave: (name: string, value: unknown) => Promise<string | null>;
  onClose: () => void;
}) {
  const [name, setName] = useState(initialName ?? "");
  const [raw, setRaw] = useState(initialValue === undefined ? "" : displayValue(initialValue));
  const [err, setErr] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const type = flagTypeOf(name.trim());
  useEffect(() => {
    if (type === "boolean" && !/^(true|false)$/i.test(raw)) setRaw("true");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [type]);
  const allowed = allowlist.includes(name.trim());
  const localErr = !name.trim()
    ? null
    : !/^[A-Za-z0-9_]+$/.test(name.trim())
      ? "Letters, digits and underscores only"
      : type === "unknown"
        ? "Unrecognized prefix (FFlag, DFFlag, FInt, DFInt, FString, DFString, FLog, DFLog)"
        : (type === "integer" || type === "log") && !/^-?\d+$/.test(raw.trim())
          ? "Enter a whole number"
          : null;
  const submit = async () => {
    setSaving(true);
    const e = await onSave(name.trim(), parseFlagInput(raw, type));
    setSaving(false);
    if (e) setErr(e);
    else onClose();
  };
  return (
    <Modal onClose={onClose}>
      <h3 className="text-[15px] font-semibold text-fg">{initialName ? "Edit flag" : "Add flag"}</h3>
      <div className="mt-4 space-y-3">
        <div>
          <label className="mb-1.5 block text-xs font-medium text-fg-2">Flag name</label>
          <Input autoFocus={!initialName} list="aeolus-allowlist" value={name} onChange={(e) => { setName(e.target.value); setErr(null); }} placeholder="e.g. DFIntTextureQualityOverride" className="font-mono" />
          <datalist id="aeolus-allowlist">
            {allowlist.map((a) => <option key={a} value={a} />)}
          </datalist>
          {name.trim() && !localErr && (
            <div className={`mt-1 text-xs ${allowed ? "text-success" : "text-warning"}`}>
              {allowed ? "On Roblox's allowlist" : "Not on Roblox's known allowlist — valid JSON, but Roblox will most likely ignore it"}
            </div>
          )}
        </div>
        <div>
          <label className="mb-1.5 block text-xs font-medium text-fg-2">Value <span className="text-fg-3">({type === "unknown" ? "any" : type})</span></label>
          {type === "boolean" ? (
            <Select value={/^false$/i.test(raw) ? "false" : "true"} onChange={(e) => setRaw(e.target.value)}>
              <option value="true">true</option>
              <option value="false">false</option>
            </Select>
          ) : (
            <Input autoFocus={!!initialName} value={raw} onChange={(e) => { setRaw(e.target.value); setErr(null); }} onKeyDown={(e) => e.key === "Enter" && !localErr && name.trim() && submit()} className="font-mono" />
          )}
        </div>
        {(localErr || err) && <div className="text-xs text-danger">{localErr ?? err}</div>}
      </div>
      <div className="mt-5 flex justify-end gap-2">
        <Button variant="ghost" onClick={onClose}>Cancel</Button>
        <Button variant="primary" loading={saving} disabled={!name.trim() || !!localErr} onClick={submit}>Save</Button>
      </div>
    </Modal>
  );
}

export function ImportFlagsDialog({ onImport, onClose }: { onImport: (json: string, replace: boolean) => Promise<string | null>; onClose: () => void }) {
  const [text, setText] = useState("");
  const [replace, setReplace] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  let parseErr: string | null = null;
  if (text.trim()) {
    try {
      const v = JSON.parse(text);
      if (!v || typeof v !== "object" || Array.isArray(v)) parseErr = "Expected a JSON object of flag names to values";
    } catch (e) {
      parseErr = "Invalid JSON: " + (e as Error).message;
    }
  }
  return (
    <Modal onClose={onClose} wide>
      <h3 className="text-[15px] font-semibold text-fg">Import Fast Flags</h3>
      <p className="mt-1 text-[13px] text-fg-2">Paste a JSON object such as the contents of a ClientAppSettings.json file.</p>
      <Textarea rows={12} className="mt-3" placeholder={'{\n  "DFIntTextureQualityOverride": 3\n}'} value={text} onChange={(e) => { setText(e.target.value); setErr(null); }} />
      {(parseErr || err) && <div className="mt-1.5 text-xs text-danger">{parseErr ?? err}</div>}
      <label className="mt-3 flex items-center gap-2 text-[13px] text-fg-2">
        <input type="checkbox" className="accent-[var(--accent)]" checked={replace} onChange={(e) => setReplace(e.target.checked)} />
        Replace all flags in this profile (otherwise merge)
      </label>
      <div className="mt-5 flex justify-end gap-2">
        <Button variant="ghost" onClick={onClose}>Cancel</Button>
        <Button
          variant="primary"
          loading={busy}
          disabled={!text.trim() || !!parseErr}
          onClick={async () => {
            setBusy(true);
            const e = await onImport(text, replace);
            setBusy(false);
            if (e) setErr(e);
            else onClose();
          }}
        >
          Import
        </Button>
      </div>
    </Modal>
  );
}

export function CompareDialog({ profiles, initialLeft, onClose }: { profiles: FlagProfileSummary[]; initialLeft: string; onClose: () => void }) {
  const [left, setLeft] = useState(initialLeft);
  const [right, setRight] = useState(profiles.find((p) => p.id !== initialLeft)?.id ?? initialLeft);
  const [cmp, setCmp] = useState<Comparison | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    setCmp(null);
    api.ffCompare(left, right).then(setCmp).catch((e) => setErr(errorMessage(e)));
  }, [left, right]);
  const name = (id: string) => profiles.find((p) => p.id === id)?.name ?? "?";
  return (
    <Modal onClose={onClose} wide>
      <h3 className="text-[15px] font-semibold text-fg">Compare profiles</h3>
      <div className="mt-3 grid grid-cols-2 gap-3">
        <Select value={left} onChange={(e) => setLeft(e.target.value)}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}</Select>
        <Select value={right} onChange={(e) => setRight(e.target.value)}>{profiles.map((p) => <option key={p.id} value={p.id}>{p.name}</option>)}</Select>
      </div>
      <div className="mt-4 max-h-[360px] overflow-auto rounded-lg border border-line text-[13px]">
        {err && <div className="p-3 text-danger">{err}</div>}
        {!cmp && !err && <div className="flex justify-center p-6"><Spinner /></div>}
        {cmp && (
          <div className="divide-y divide-line">
            <div className="px-3 py-2 text-xs text-fg-3">{cmp.same} identical · {cmp.changed.length} different · {cmp.onlyLeft.length} only in {name(left)} · {cmp.onlyRight.length} only in {name(right)}</div>
            {cmp.changed.map(([k, a, b]) => (
              <div key={k} className="grid grid-cols-[1fr_auto_auto] gap-3 px-3 py-1.5 font-mono text-xs">
                <span className="truncate text-fg">{k}</span>
                <span className="text-warning">{displayValue(a)}</span>
                <span className="text-accent">{displayValue(b)}</span>
              </div>
            ))}
            {cmp.onlyLeft.map((k) => <div key={"l" + k} className="px-3 py-1.5 font-mono text-xs text-fg-2">− {k} <span className="text-fg-3">(only in {name(left)})</span></div>)}
            {cmp.onlyRight.map((k) => <div key={"r" + k} className="px-3 py-1.5 font-mono text-xs text-fg-2">+ {k} <span className="text-fg-3">(only in {name(right)})</span></div>)}
          </div>
        )}
      </div>
      <div className="mt-5 flex justify-end"><Button variant="primary" onClick={onClose}>Done</Button></div>
    </Modal>
  );
}
