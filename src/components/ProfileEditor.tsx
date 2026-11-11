import { useState } from "react";
import { Flame, Gamepad2, Monitor, Rocket, Shield, Sparkles, Star, User, Wind, Zap, type LucideIcon } from "lucide-react";
import { Modal } from "../stores/dialogs";
import type { FlagProfileSummary, Profile, ProfileInput, SkyPack } from "../types";
import { Button, Input, Select, Toggle, Field, cx } from "./ui";
import { isHexColor } from "../utils/format";

export const PROFILE_ICONS: Record<string, LucideIcon> = {
  wind: Wind,
  "gamepad-2": Gamepad2,
  zap: Zap,
  sparkles: Sparkles,
  monitor: Monitor,
  rocket: Rocket,
  flame: Flame,
  shield: Shield,
  star: Star,
  user: User,
};

export function ProfileIcon({ name, size = 16, className }: { name: string; size?: number; className?: string }) {
  const I = PROFILE_ICONS[name] ?? Wind;
  return <I size={size} className={className} />;
}

export function ProfileEditor({
  initial,
  flagProfiles,
  skies = [],
  onSave,
  onClose,
}: {
  initial?: Profile;
  flagProfiles: FlagProfileSummary[];
  skies?: SkyPack[];
  onSave: (p: ProfileInput) => Promise<boolean>;
  onClose: () => void;
}) {
  const [p, setP] = useState<ProfileInput>(
    initial
      ? { name: initial.name, description: initial.description, icon: initial.icon, accent: initial.accent, fastFlagProfileId: initial.fastFlagProfileId, sky: initial.sky ?? null, launch: { ...initial.launch } }
      : { name: "", description: "", icon: "gamepad-2", accent: "#FF861F", fastFlagProfileId: null, sky: null, launch: { applyFastFlags: true, updateBeforeLaunch: true, verifyBeforeLaunch: false, applyMods: true, afterLaunch: "nothing" } },
  );
  const [saving, setSaving] = useState(false);
  const nameErr = !p.name.trim() ? "Name is required" : p.name.length > 60 ? "Too long" : null;
  const colorErr = !isHexColor(p.accent) ? "Use a hex color like #FF861F" : null;
  const setLaunch = (k: keyof ProfileInput["launch"], v: unknown) => setP({ ...p, launch: { ...p.launch, [k]: v } });

  return (
    <Modal onClose={onClose} wide>
      <h3 className="text-[15px] font-semibold text-fg">{initial ? "Edit profile" : "New profile"}</h3>
      <div className="mt-4 grid grid-cols-2 gap-4">
        <div>
          <label className="mb-1.5 block text-xs font-medium text-fg-2">Name</label>
          <Input autoFocus value={p.name} onChange={(e) => setP({ ...p, name: e.target.value })} placeholder="e.g. Competitive" />
        </div>
        <div>
          <label className="mb-1.5 block text-xs font-medium text-fg-2">Fast Flag profile</label>
          <Select value={p.fastFlagProfileId ?? ""} onChange={(e) => setP({ ...p, fastFlagProfileId: e.target.value || null })}>
            <option value="">Use the default Fast Flag profile</option>
            {flagProfiles.map((f) => (
              <option key={f.id} value={f.id}>{f.name} ({f.count} flags)</option>
            ))}
          </Select>
        </div>
        <div>
          <label className="mb-1.5 block text-xs font-medium text-fg-2">Sky</label>
          <Select value={p.sky ?? ""} onChange={(e) => setP({ ...p, sky: e.target.value || null })} aria-label="Sky">
            <option value="">Use the sky from Customization</option>
            <option value="default">Roblox default sky</option>
            {skies.map((s) => (
              <option key={s.id} value={s.id}>{s.name}</option>
            ))}
          </Select>
        </div>
        <div>
          <label className="mb-1.5 block text-xs font-medium text-fg-2">Description</label>
          <Input value={p.description} maxLength={300} onChange={(e) => setP({ ...p, description: e.target.value })} placeholder="Optional" />
        </div>
        <div>
          <label className="mb-1.5 block text-xs font-medium text-fg-2">Icon</label>
          <div className="flex flex-wrap gap-1.5">
            {Object.keys(PROFILE_ICONS).map((k) => (
              <button
                key={k}
                aria-label={k}
                onClick={() => setP({ ...p, icon: k })}
                className={cx("flex h-8 w-8 items-center justify-center rounded-lg border transition-colors", p.icon === k ? "border-accent bg-accent/10 text-accent" : "border-line text-fg-2 hover:text-fg")}
              >
                <ProfileIcon name={k} size={15} />
              </button>
            ))}
          </div>
        </div>
        <div>
          <label className="mb-1.5 block text-xs font-medium text-fg-2">Accent color</label>
          <div className="flex items-center gap-2">
            <input type="color" aria-label="Pick accent color" value={isHexColor(p.accent) ? p.accent : "#ff861f"} onChange={(e) => setP({ ...p, accent: e.target.value.toUpperCase() })} className="h-9 w-10 cursor-pointer rounded-lg border border-line bg-transparent" />
            <Input value={p.accent} onChange={(e) => setP({ ...p, accent: e.target.value })} className="w-28 font-mono" />
          </div>
          {colorErr && <div className="mt-1 text-xs text-danger">{colorErr}</div>}
        </div>
      </div>

      <div className="mt-5 divide-y divide-line rounded-xl border border-line px-4">
        <Field label="Apply Fast Flags on launch" description="Writes the selected Fast Flag profile to the installation before starting Roblox.">
          <Toggle checked={p.launch.applyFastFlags} onChange={(v) => setLaunch("applyFastFlags", v)} />
        </Field>
        <Field label="Use mods" description="Apply your sky, sounds, cursors and fonts from the Mods page. Turn off for plain Roblox.">
          <Toggle checked={p.launch.applyMods ?? true} onChange={(v) => setLaunch("applyMods", v)} />
        </Field>
        <Field label="Update before launch" description="Installs a pending Roblox update first (Aeolus installation only).">
          <Toggle checked={p.launch.updateBeforeLaunch} onChange={(v) => setLaunch("updateBeforeLaunch", v)} />
        </Field>
        <Field label="Verify files before launch" description="Quick size check of every file; adds a moment to each launch.">
          <Toggle checked={p.launch.verifyBeforeLaunch} onChange={(v) => setLaunch("verifyBeforeLaunch", v)} />
        </Field>
        <Field label="After Roblox starts">
          <Select value={p.launch.afterLaunch} onChange={(e) => setLaunch("afterLaunch", e.target.value)} className="w-40">
            <option value="nothing">Keep Aeolus open</option>
            <option value="minimize">Minimize Aeolus</option>
            <option value="close">Close Aeolus</option>
          </Select>
        </Field>
      </div>

      <div className="mt-5 flex justify-end gap-2">
        <Button variant="ghost" onClick={onClose}>Cancel</Button>
        <Button
          variant="primary"
          loading={saving}
          disabled={!!nameErr || !!colorErr}
          onClick={async () => {
            setSaving(true);
            const ok = await onSave({ ...p, accent: p.accent.toUpperCase() });
            setSaving(false);
            if (ok) onClose();
          }}
        >
          {initial ? "Save changes" : "Create profile"}
        </Button>
      </div>
    </Modal>
  );
}
