import { motion } from "framer-motion";
import { api } from "../services/api";
import { Blocks, MessageCircle, Download, Flag, Wand2, Home, Paintbrush, PanelLeftClose, PanelLeftOpen, Rocket, Settings as SettingsIcon, Stethoscope } from "lucide-react";
import type { PageId } from "../types";
import { useApp } from "../stores/app";
import { SearchBox } from "./SearchBox";
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

  const item = (n: (typeof NAV)[number]) => {
    const active = page === n.id;
    const Icon = n.icon;
    return (
      <button
        key={n.id}
        onClick={() => navigate(n.id)}
        title={collapsed ? n.label : undefined}
        aria-current={active ? "page" : undefined}
        className={cx(
          "relative flex items-center gap-3 rounded-md px-3 text-[13.5px] transition-colors",
          compact ? "h-8" : "h-9",
          active ? "bg-fg/[0.07] text-fg" : "text-fg-2 hover:bg-fg/[0.04] hover:text-fg",
          collapsed && "justify-center px-0",
        )}
      >
        {active && <motion.span layoutId="nav-indicator" transition={{ duration: 0.18 }} className="absolute left-0 top-1/2 h-4 w-[3px] -translate-y-1/2 rounded-full bg-accent" />}
        <Icon size={17} strokeWidth={1.75} className={cx("shrink-0", active ? "text-fg" : "text-fg-2")} />
        {!collapsed && <span className="truncate">{n.label}</span>}
      </button>
    );
  };

  return (
    <motion.aside
      animate={{ width: collapsed ? 60 : 248 }}
      transition={{ duration: 0.18, ease: "easeOut" }}
      className={cx("flex shrink-0 flex-col", subtle ? "bg-bg" : "bg-bg-2")}
    >
      <div className={cx("px-3 pb-3 pt-2", collapsed && "px-2")}>
        <SearchBox collapsed={collapsed} />
      </div>

      <nav className={cx("flex flex-1 flex-col gap-0.5 overflow-y-auto px-3", collapsed && "px-2")} aria-label="Main">
        {NAV.filter((n) => n.id !== "settings").map(item)}
        <div className="flex-1" />
        {item(NAV.find((n) => n.id === "settings")!)}
      </nav>

      <div className={cx("flex items-center gap-1 px-3 py-2.5", collapsed && "flex-col px-2")}>
        {!collapsed && <span className="flex-1 truncate pl-1 text-[11px] text-fg-3">Version {info?.version ?? "…"}</span>}
        <button
          aria-label="Join the Aeolus Discord"
          title="Join the Aeolus Discord"
          className="flex h-7 w-7 items-center justify-center rounded-md text-fg-3 hover:bg-fg/[0.06] hover:text-[#5865F2]"
          onClick={() => api.openLink("discord").catch(() => undefined)}
        >
          <MessageCircle size={15} />
        </button>
        <button
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          className="flex h-7 w-7 items-center justify-center rounded-md text-fg-3 hover:bg-fg/[0.06] hover:text-fg"
          onClick={() => updateSettings((s) => ({ ...s, general: { ...s.general, sidebarCollapsed: !collapsed } }))}
        >
          {collapsed ? <PanelLeftOpen size={15} /> : <PanelLeftClose size={15} />}
        </button>
      </div>
    </motion.aside>
  );
}
