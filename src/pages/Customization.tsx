import { Check, Monitor, Moon, Sun } from "lucide-react";
import { useEffect, useState } from "react";
import { useApp } from "../stores/app";
import { Card, Field, Input, PageHeader, SectionTitle, Tabs, Toggle, cx } from "../components/ui";
import { Logo } from "../components/Logo";
import { isHexColor } from "../utils/format";
import type { Settings } from "../types";

const ACCENTS = ["#FF861F", "#FFAB45", "#F25F4C", "#E5484D", "#3E8EF7", "#14B8A6", "#36B37E", "#A970FF"];

export default function Customization() {
  const { settings, updateSettings } = useApp();
  const [accentText, setAccentText] = useState(settings?.general.accent ?? "#FF861F");
  useEffect(() => setAccentText(settings?.general.accent ?? "#FF861F"), [settings?.general.accent]);
  if (!settings) return null;
  const s = settings;
  const set = <K extends keyof Settings>(section: K, patch: Partial<Settings[K]>) =>
    updateSettings((cur) => ({ ...cur, [section]: { ...(cur[section] as object), ...patch } }));

  return (
    <div className="mx-auto max-w-4xl px-8 py-8">
      <PageHeader title="Customization" description="Make Aeolus look and behave the way you like. Changes apply instantly and are saved." />

      <SectionTitle>Theme</SectionTitle>
      <div className="mb-8 grid grid-cols-3 gap-3">
        {([
          { id: "dark", label: "Dark", icon: Moon },
          { id: "light", label: "Light", icon: Sun },
          { id: "system", label: "System default", icon: Monitor },
        ] as const).map((t) => (
          <button
            key={t.id}
            onClick={() => set("general", { theme: t.id })}
            className={cx("flex items-center gap-3 rounded-xl border bg-surface p-4 text-left transition-colors", s.general.theme === t.id ? "border-accent/60" : "border-line hover:bg-surface-hover")}
          >
            <t.icon size={18} className={s.general.theme === t.id ? "text-accent" : "text-fg-3"} />
            <span className="flex-1 text-[13px] font-medium text-fg">{t.label}</span>
            {s.general.theme === t.id && <Check size={15} className="text-accent" />}
          </button>
        ))}
      </div>

      <SectionTitle>Accent color</SectionTitle>
      <Card className="mb-8 p-4">
        <div className="flex flex-wrap items-center gap-2.5">
          {ACCENTS.map((c) => (
            <button
              key={c}
              aria-label={`Accent ${c}`}
              onClick={() => set("general", { accent: c })}
              className={cx("flex h-8 w-8 items-center justify-center rounded-full ring-offset-2 ring-offset-[var(--surface)] transition", s.general.accent.toUpperCase() === c ? "ring-2 ring-fg" : "hover:scale-110")}
              style={{ background: c }}
            >
              {s.general.accent.toUpperCase() === c && <Check size={14} className="text-black/70" />}
            </button>
          ))}
          <div className="ml-auto flex items-center gap-2">
            <input
              type="color"
              aria-label="Custom accent color"
              value={isHexColor(s.general.accent) ? s.general.accent : "#ff861f"}
              onChange={(e) => set("general", { accent: e.target.value.toUpperCase() })}
              className="h-8 w-10 cursor-pointer rounded-md border border-line bg-transparent"
            />
            <Input
              value={accentText}
              onChange={(e) => setAccentText(e.target.value)}
              onBlur={() => (isHexColor(accentText) ? set("general", { accent: accentText.toUpperCase() }) : setAccentText(s.general.accent))}
              className="w-28 font-mono"
              aria-label="Accent hex value"
            />
            <button className="text-xs text-fg-3 hover:text-fg" onClick={() => set("general", { accent: "#FF861F" })}>Reset</button>
          </div>
        </div>
      </Card>

      <SectionTitle>Interface</SectionTitle>
      <Card className="mb-8 divide-y divide-line px-4">
        <Field label="Sidebar appearance" description="Solid uses a separate surface; subtle blends with the background.">
          <Tabs value={s.appearance.sidebarStyle} onChange={(v) => set("appearance", { sidebarStyle: v })} items={[{ id: "solid", label: "Solid" }, { id: "subtle", label: "Subtle" }]} />
        </Field>
        <Field label="Collapsed sidebar" description="Show icons only.">
          <Toggle checked={s.general.sidebarCollapsed} onChange={(v) => set("general", { sidebarCollapsed: v })} />
        </Field>
        <Field label="Compact navigation" description="Tighter spacing in the sidebar.">
          <Toggle checked={s.appearance.compactNavigation} onChange={(v) => set("appearance", { compactNavigation: v })} />
        </Field>
        <Field label="Dashboard background">
          <Tabs value={s.appearance.dashboardBackground} onChange={(v) => set("appearance", { dashboardBackground: v })} items={[{ id: "plain", label: "Plain" }, { id: "glow", label: "Glow" }, { id: "grid", label: "Grid" }]} />
        </Field>
        <Field
          label="Logo presentation"
          description="Show the wind mark with or without the Aeolus name in the sidebar."
        >
          <div className="mr-2 flex items-center gap-2 rounded-lg border border-line px-2.5 py-1.5">
            <Logo size={18} />
            {s.appearance.logoPresentation === "full" && <span className="text-xs font-semibold text-fg">Aeolus</span>}
          </div>
          <Tabs value={s.appearance.logoPresentation} onChange={(v) => set("appearance", { logoPresentation: v })} items={[{ id: "full", label: "Mark + name" }, { id: "icon", label: "Mark only" }]} />
        </Field>
      </Card>

      <SectionTitle>Motion</SectionTitle>
      <Card className="mb-8 divide-y divide-line px-4">
        <Field label="Animations" description="Reduced keeps essential feedback only; disabled turns off all transitions.">
          <Tabs value={s.appearance.animations} onChange={(v) => set("appearance", { animations: v })} items={[{ id: "enabled", label: "Enabled" }, { id: "reduced", label: "Reduced" }, { id: "disabled", label: "Disabled" }]} />
        </Field>
      </Card>

      <SectionTitle>Window</SectionTitle>
      <Card className="divide-y divide-line px-4">
        <Field label="Close to system tray" description="The close button hides Aeolus to the tray instead of exiting.">
          <Toggle checked={s.general.minimizeToTray} onChange={(v) => set("general", { minimizeToTray: v })} />
        </Field>
        <Field label="Start minimized" description="Open in the tray without showing the window.">
          <Toggle checked={s.general.startMinimized} onChange={(v) => set("general", { startMinimized: v })} />
        </Field>
        <Field label="Window size and position" description="Aeolus remembers the window size, position and maximized state automatically.">
          <span className="text-xs text-fg-3">Automatic</span>
        </Field>
      </Card>
    </div>
  );
}
