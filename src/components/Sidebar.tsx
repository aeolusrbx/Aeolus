import { motion } from "framer-motion";
import { Blocks, Download, Flag, Wand2, Home, Paintbrush, PanelLeftClose, PanelLeftOpen, Rocket, Settings as SettingsIcon, Stethoscope } from "lucide-react";
import type { PageId } from "../types";
import { useApp } from "../stores/app";
import { Logo } from "./Logo";
import { cx } from "./ui";

export const NAV: { id: PageId; label: string; icon: typeof Home }[] = [
  { id: "home", label: "Home", icon: Home },
  { id: "installations", label: "Installations", icon: Download },
  { id: "launch", label: "Launch", icon: Rocket },
  { id: "fastflags", label: "Fast Flags", icon: Flag },
  { id: "mods", label: "Mods", icon: Wand2 },
  { id: "customization", label: "Customization", icon: Paintbrush },
  { id: "plugins", label: "Plugins", icon: Blocks },
  { id: "diagnostics", label: "Diagnostics", icon: Stethoscope },
  { id: "settings", label: "Settings", icon: SettingsIcon },
];

export function Sidebar() {
  const { page, navigate, settings, updateSettings, info } = useApp();
  const collapsed = settings?.general.sidebarCollapsed ?? false;
  const compact = settings?.appearance.compactNavigation ?? false;
  const subtle = settings?.appearance.sidebarStyle === "subtle";
  const iconOnlyLogo = settings?.appearance.logoPresentation === "icon";

  return (
    <motion.aside
      animate={{ width: collapsed ? 64 : 216 }}
      transition={{ duration: 0.18, ease: "easeOut" }}
      className={cx("flex shrink-0 flex-col border-r border-line", subtle ? "bg-bg" : "bg-bg-2")}
    >
      <div className={cx("flex items-center gap-2.5 px-4", compact ? "h-14" : "h-16")}>
        <Logo size={collapsed ? 30 : 28} className="shrink-0" />
        {!collapsed && !iconOnlyLogo && <span className="truncate text-[15px] font-semibold tracking-tight text-fg">Aeolus</span>}
      </div>

      <nav className="flex flex-1 flex-col gap-0.5 px-2.5" aria-label="Main">
        {NAV.map((n) => {
          const active = page === n.id;
          const Icon = n.icon;
          return (
            <button
              key={n.id}
              onClick={() => navigate(n.id)}
              title={collapsed ? n.label : undefined}
              aria-current={active ? "page" : undefined}
              className={cx(
                "group relative flex items-center gap-3 rounded-lg px-2.5 text-[13px] font-medium transition-colors",
                compact ? "h-8" : "h-9",
                active ? "bg-surface text-fg" : "text-fg-2 hover:bg-surface/60 hover:text-fg",
                collapsed && "justify-center px-0",
              )}
            >
              {active && <motion.span layoutId="nav-indicator" className="absolute left-0 top-1/2 h-4 w-[3px] -translate-y-1/2 rounded-full bg-accent" />}
              <Icon size={17} className={cx("shrink-0", active ? "text-accent" : "text-fg-3 group-hover:text-fg-2")} />
              {!collapsed && <span className="truncate">{n.label}</span>}
            </button>
          );
        })}
      </nav>

      <div className={cx("flex items-center gap-2 border-t border-line px-3 py-2.5", collapsed && "flex-col")}>
        {!collapsed && <span className="flex-1 truncate text-[11px] text-fg-3">v{info?.version ?? "…"}</span>}
        <button
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          className="flex h-7 w-7 items-center justify-center rounded-md text-fg-3 hover:bg-surface hover:text-fg"
          onClick={() => updateSettings((s) => ({ ...s, general: { ...s.general, sidebarCollapsed: !collapsed } }))}
        >
          {collapsed ? <PanelLeftOpen size={15} /> : <PanelLeftClose size={15} />}
        </button>
      </div>
    </motion.aside>
  );
}
