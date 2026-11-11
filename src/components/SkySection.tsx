import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { Check, CloudSun, FileArchive, FolderOpen, Info, MoreHorizontal, Pencil, Trash2 } from "lucide-react";
import { useState } from "react";
import { api } from "../services/api";
import { useResource } from "../hooks/useResource";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import type { SkyPack } from "../types";
import { Button, Card, IconButton, SectionTitle, Spinner, cx } from "./ui";

/** A stand-in tile for packs without a preview image. */
function SkyArt({ pack }: { pack?: SkyPack }) {
  if (pack?.previewUrl) return <img src={pack.previewUrl} alt="" className="h-full w-full object-cover" draggable={false} />;
  return (
    <div className={cx("flex h-full w-full items-center justify-center", pack ? "bg-gradient-to-br from-[#3b6fb6] via-[#7aa9dd] to-[#f2b46b]" : "bg-gradient-to-b from-[#6aa6e8] to-[#cfe5fb]")}>
      <CloudSun size={26} className="text-white/85 drop-shadow" />
    </div>
  );
}

export function SkySection() {
  const sky = useResource(api.skiesOverview, ["skies", "install", "settings"]);
  const dialogs = useDialogs();
  const toast = useToast();
  const [busy, setBusy] = useState<string | null>(null);
  const [menu, setMenu] = useState<string | null>(null);
  const data = sky.data;

  const choose = async (id: string | null) => {
    setBusy(id ?? "default");
    try {
      const name = await api.skySetGlobal(id);
      toast.success(id ? `Sky set to ${name}` : "Using Roblox's default sky", "Applies to Roblox now, or on your next launch.");
      await sky.reload();
    } catch (e) {
      dialogs.showError("Could not change the sky", e);
    } finally {
      setBusy(null);
    }
  };

  const add = async (directory: boolean) => {
    const picked = await openDialog(
      directory
        ? { directory: true, title: "Choose a folder with the sky's .tex files" }
        : { filters: [{ name: "Sky pack (.zip)", extensions: ["zip"] }], title: "Choose a sky pack .zip" },
    ).catch(() => null);
    if (typeof picked !== "string") return;
    try {
      const p = await api.skyImport(picked, null);
      toast.success(`Added sky "${p.name}"`, `${p.files.length} texture files`);
      await sky.reload();
    } catch (e) {
      dialogs.showError("Could not add this sky", e);
    }
  };

  const selected = data?.globalPackId ?? null;

  return (
    <div className="mb-8">
      <SectionTitle
        action={
          <div className="flex gap-1">
            <Button size="sm" variant="ghost" icon={<FolderOpen size={13} />} onClick={() => add(true)}>Add folder</Button>
            <Button size="sm" variant="ghost" icon={<FileArchive size={13} />} onClick={() => add(false)}>Add .zip</Button>
          </div>
        }
      >
        Sky
      </SectionTitle>
      <Card className="p-4">
        {!data && <div className="flex justify-center py-6"><Spinner /></div>}
        {data && (
          <div className="grid grid-cols-4 gap-3">
            {[null, ...data.packs].map((p) => {
              const id = p?.id ?? null;
              const active = selected === id;
              return (
                <div key={id ?? "default"} className="relative">
                  <button
                    onClick={() => !active && choose(id)}
                    aria-pressed={active}
                    aria-label={p ? `Sky ${p.name}` : "Roblox default sky"}
                    className={cx("block w-full overflow-hidden rounded-xl border text-left transition-colors", active ? "border-accent ring-1 ring-accent/50" : "border-line hover:border-fg-3")}
                  >
                    <div className="relative h-20 w-full">
                      <SkyArt pack={p ?? undefined} />
                      {active && (
                        <span className="absolute right-1.5 top-1.5 flex h-5 w-5 items-center justify-center rounded-full bg-accent text-on-accent"><Check size={12} /></span>
                      )}
                      {busy === (id ?? "default") && <span className="absolute inset-0 flex items-center justify-center bg-black/30"><Spinner /></span>}
                    </div>
                    <div className="truncate px-2.5 py-2 text-xs font-medium text-fg">{p ? p.name : "Roblox default"}</div>
                  </button>
                  {p && (
                    <div className="absolute bottom-1 right-1">
                      <IconButton label={`Options for ${p.name}`} className="h-6 w-6" onClick={() => setMenu(menu === p.id ? null : p.id)}>
                        <MoreHorizontal size={14} />
                      </IconButton>
                      {menu === p.id && (
                        <>
                          <div className="fixed inset-0 z-10" onClick={() => setMenu(null)} />
                          <div className="absolute right-0 top-7 z-20 w-36 overflow-hidden rounded-xl border border-line bg-surface py-1 shadow-[var(--shadow)]">
                            <button
                              className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-fg hover:bg-surface-hover"
                              onClick={async () => {
                                setMenu(null);
                                const name = await dialogs.prompt({ title: "Rename sky", initial: p.name, validate: (v) => (!v.trim() ? "Required" : v.length > 60 ? "Too long" : null) });
                                if (name) api.skyRename(p.id, name).then(() => sky.reload()).catch((e) => dialogs.showError("Could not rename", e));
                              }}
                            >
                              <Pencil size={12} /> Rename
                            </button>
                            <button
                              className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs text-danger hover:bg-danger/10"
                              onClick={async () => {
                                setMenu(null);
                                const ok = await dialogs.confirm({
                                  title: `Delete "${p.name}"?`,
                                  description: "The sky is removed from Aeolus. If it's in use, Roblox's own sky is put back, and profiles using it switch to the global sky.",
                                  confirmLabel: "Delete",
                                  tone: "danger",
                                });
                                if (ok) api.skyDelete(p.id).then(() => { toast.success("Sky deleted"); return sky.reload(); }).catch((e) => dialogs.showError("Could not delete", e));
                              }}
                            >
                              <Trash2 size={12} /> Delete
                            </button>
                          </div>
                        </>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
        {data && data.packs.length === 0 && (
          <p className="mt-3 text-xs text-fg-2">
            Add a sky pack: a folder or .zip with the six Roblox sky textures (<span className="font-mono">sky512_bk, _dn, _ft, _lf, _rt, _up .tex</span>). Include a .png or .jpg in the pack to get a preview.
          </p>
        )}
        <div className="mt-3 flex items-start gap-2 text-[11px] text-fg-3">
          <Info size={12} className="mt-0.5 shrink-0" />
          <span>
            Profiles can pick a different sky on the Launch page.
            {data?.appliedPackName ? ` Currently applied: ${data.appliedPackName}.` : ""}
          </span>
        </div>
      </Card>
    </div>
  );
}
