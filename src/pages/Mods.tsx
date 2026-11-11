import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FileUp, Info, MousePointer2, RotateCcw, Type, Volume2 } from "lucide-react";
import { api } from "../services/api";
import { useResource } from "../hooks/useResource";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import { SkySection } from "../components/SkySection";
import { Badge, Button, Card, PageHeader, SectionTitle, Spinner, Toggle, cx } from "../components/ui";
import type { ModKind, ModSlot } from "../types";

const EXT: Record<ModKind, string[]> = { sound: ["ogg", "mp3", "wav"], cursor: ["png"], font: ["ttf", "otf"] };
const KIND_ICON = { sound: Volume2, cursor: MousePointer2, font: Type };

function Checker({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-line"
      style={{ backgroundColor: "#7a8796", backgroundImage: "linear-gradient(45deg,#6b7785 25%,transparent 25%,transparent 75%,#6b7785 75%),linear-gradient(45deg,#6b7785 25%,transparent 25%,transparent 75%,#6b7785 75%)", backgroundSize: "12px 12px", backgroundPosition: "0 0,6px 6px" }}
    >
      {children}
    </div>
  );
}

function SlotRow({ slot, reload }: { slot: ModSlot; reload: () => Promise<void> }) {
  const dialogs = useDialogs();
  const toast = useToast();
  const c = slot.choice;
  const Icon = KIND_ICON[slot.kind];
  const presetLabel = c?.preset ? slot.presets.find((p) => p.id === c.preset)?.label : null;
  const current = c ? (presetLabel ?? c.originalName ?? "Custom file") : null;

  const run = async (fn: () => Promise<unknown>, ok: string) => {
    try {
      await fn();
      toast.success(ok, "Applied to Roblox. Restart Roblox if it's open.");
      await reload();
    } catch (e) {
      dialogs.showError(`Could not change ${slot.label.toLowerCase()}`, e);
    }
  };

  return (
    <div className="flex items-center gap-4 px-4 py-3">
      {slot.kind === "cursor" ? (
        <Checker>{slot.previewUrl ? <img src={slot.previewUrl} alt="" className="h-8 w-8 [image-rendering:pixelated]" /> : <MousePointer2 size={18} className="text-white/80" />}</Checker>
      ) : (
        <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-lg bg-surface-hover text-fg-3"><Icon size={18} /></div>
      )}
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-[13px] font-medium text-fg">{slot.label}</span>
          {current ? <Badge tone={c?.enabled ? "accent" : "neutral"}>{current}</Badge> : <span className="text-[11px] text-fg-3">Roblox original</span>}
        </div>
        <div className="truncate text-xs text-fg-3">{slot.description}</div>
        {slot.presets.length > 0 && (
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {slot.presets.map((p) => {
              const prev = slot.presetPreviews.find(([id]) => id === p.id)?.[1];
              const active = c?.preset === p.id;
              return (
                <button
                  key={p.id}
                  onClick={() => !active && run(() => api.modSetPreset(slot.id, p.id), `${slot.label}: ${p.label}`)}
                  className={cx("inline-flex h-7 items-center gap-1.5 rounded-md border px-2 text-xs transition-colors", active ? "border-accent/60 bg-accent/10 text-fg" : "border-line text-fg-2 hover:text-fg")}
                >
                  {prev && <img src={prev} alt="" className="h-4 w-4 rounded-sm bg-[#7a8796]" />}
                  {p.label}
                </button>
              );
            })}
          </div>
        )}
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <Button
          size="sm"
          icon={<FileUp size={13} />}
          onClick={async () => {
            const f = await openDialog({ filters: [{ name: slot.label, extensions: EXT[slot.kind] }], title: `Choose a file for ${slot.label}` }).catch(() => null);
            if (typeof f === "string") run(() => api.modSetFile(slot.id, f), `${slot.label} updated`);
          }}
        >
          Choose file
        </Button>
        {c && (
          <>
            <Toggle label={`${slot.label} on`} checked={c.enabled} onChange={(v) => run(() => api.modSetEnabled(slot.id, v), `${slot.label} ${v ? "on" : "off"}`)} />
            <button title="Back to Roblox's original" aria-label={`Reset ${slot.label}`} className="flex h-7 w-7 items-center justify-center rounded-md text-fg-3 hover:bg-surface-hover hover:text-fg" onClick={() => run(() => api.modClear(slot.id), `${slot.label} reset`)}>
              <RotateCcw size={14} />
            </button>
          </>
        )}
      </div>
    </div>
  );
}

export default function Mods() {
  const mods = useResource(api.modsOverview, ["mods", "install"]);
  const dialogs = useDialogs();
  const toast = useToast();
  const reload = () => mods.reload();
  const group = (kind: ModKind) => mods.data?.filter((s) => s.kind === kind) ?? [];
  const anyActive = mods.data?.some((s) => s.choice) ?? false;

  return (
    <div className="mx-auto max-w-4xl px-8 py-8">
      <PageHeader
        title="Mods"
        description="Change Roblox's built-in sky, sounds, cursors and fonts. Originals are saved and can be restored any time."
        actions={
          anyActive ? (
            <Button
              icon={<RotateCcw size={15} />}
              onClick={async () => {
                const ok = await dialogs.confirm({ title: "Reset all mods?", description: "Every sound, cursor and font goes back to Roblox's original. Your sky is not affected.", confirmLabel: "Reset all", tone: "warning" });
                if (ok) api.modsResetAll().then(() => { toast.success("All mods reset"); return reload(); }).catch((e) => dialogs.showError("Could not reset mods", e));
              }}
            >
              Reset all
            </Button>
          ) : undefined
        }
      />

      <div className="mb-6 flex items-start gap-3 rounded-xl border border-line bg-surface px-4 py-3 text-xs text-fg-2">
        <Info size={15} className="mt-0.5 shrink-0 text-accent" />
        <div>
          Mods only change Roblox's <span className="text-fg">built-in</span> versions. A game that uses its own sky, sounds or cursor keeps those. Changes apply at launch; if Roblox is already open, restart it to see them. Profiles can turn mods off on the Launch page.
        </div>
      </div>

      <SkySection />

      {!mods.data && <div className="flex justify-center py-10"><Spinner /></div>}
      {mods.data && (
        <>
          <SectionTitle>Sounds</SectionTitle>
          <Card className="mb-8 divide-y divide-line">{group("sound").map((s) => <SlotRow key={s.id} slot={s} reload={reload} />)}</Card>
          <SectionTitle>Cursors</SectionTitle>
          <Card className="mb-8 divide-y divide-line">{group("cursor").map((s) => <SlotRow key={s.id} slot={s} reload={reload} />)}</Card>
          <SectionTitle>Fonts</SectionTitle>
          <Card className="divide-y divide-line">{group("font").map((s) => <SlotRow key={s.id} slot={s} reload={reload} />)}</Card>
        </>
      )}
    </div>
  );
}
