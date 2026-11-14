import settingsSource from "../pages/Settings.tsx?raw";
import type { PageId } from "../types";

export interface SearchEntry {
  title: string;
  /** Breadcrumb shown under the title, e.g. ["Settings", "Launch"]. */
  path: string[];
  page: PageId;
  section?: string;
  /** A Field label to scroll to and highlight. */
  setting?: string;
  description?: string;
  keywords?: string;
  kind: "page" | "setting" | "feature";
}

const PAGES: { page: PageId; title: string; keywords: string }[] = [
  { page: "home", title: "Home", keywords: "dashboard start play launch status recent games activity" },
  { page: "installations", title: "Installations", keywords: "install update verify repair reinstall uninstall roblox version folder location files" },
  { page: "launch", title: "Launch", keywords: "launch profiles game history rejoin play profile" },
  { page: "fastflags", title: "Fast Flags", keywords: "fflags flags fastflags allowlist import export presets clientappsettings" },
  { page: "mods", title: "Mods", keywords: "sky skybox sounds oof death sound cursor fonts textures" },
  { page: "customization", title: "Customization", keywords: "appearance theme accent color sidebar logo background animations" },
  { page: "plugins", title: "Plugins", keywords: "plugins add-ons extensions permissions" },
  { page: "diagnostics", title: "Diagnostics", keywords: "logs report problems troubleshoot system info crash" },
  { page: "settings", title: "Settings", keywords: "options preferences configuration" },
];

/** Things on pages other than Settings that people look for. */
const FEATURES: Omit<SearchEntry, "kind">[] = [
  { title: "Sky", path: ["Mods"], page: "mods", keywords: "skybox custom sky texture pack" },
  { title: "Death sound", path: ["Mods", "Sounds"], page: "mods", keywords: "oof classic sound" },
  { title: "Cursor", path: ["Mods", "Cursors"], page: "mods", keywords: "mouse pointer crosshair" },
  { title: "Fonts", path: ["Mods", "Fonts"], page: "mods", keywords: "font text" },
  { title: "Launch profiles", path: ["Launch"], page: "launch", keywords: "profile create edit default" },
  { title: "Game history", path: ["Launch"], page: "launch", keywords: "games played rejoin server location recent" },
  { title: "Verify files", path: ["Installations"], page: "installations", keywords: "repair check integrity broken" },
  { title: "Reinstall Roblox", path: ["Installations"], page: "installations", keywords: "reinstall fresh" },
  { title: "Install location", path: ["Installations"], page: "installations", keywords: "folder directory path move" },
  { title: "Fast Flag profiles", path: ["Fast Flags"], page: "fastflags", keywords: "flags profile import export" },
  { title: "Accent color", path: ["Customization"], page: "customization", keywords: "color orange theme" },
  { title: "Diagnostic report", path: ["Diagnostics"], page: "diagnostics", keywords: "export report logs support" },
  { title: "Logs", path: ["Diagnostics"], page: "diagnostics", keywords: "log files errors" },
];

/** Extra words people might type for some settings. */
const SETTING_KEYWORDS: Record<string, string> = {
  "After a disconnect": "rejoin reconnect auto rejoin kicked disconnected error 277",
  "FPS cap": "frame rate framerate fps limit unlock unlocker max",
  "Discord Rich Presence": "status activity rpc",
  "Show the game you're playing": "discord status game",
  "Theme": "dark mode light mode appearance",
  "Show the launch menu when Aeolus opens": "launcher menu startup small window open",
  "Handle Roblox links": "website play button protocol roblox-player",
  "Crash help": "crashed crash closed unexpectedly",
  "Launch at Windows startup": "boot autostart",
  "Release feed URL": "self update aeolus updates",
  "Delete data": "clear reset wipe",
};

const unescape = (t: string) => t.replace(/\\u([0-9a-fA-F]{4})/g, (_, h) => String.fromCharCode(parseInt(h, 16))).replace(/\\"/g, '"');

/** Read the Settings page's sections, groups and fields straight from its source, so search never goes stale. */
export function settingsEntries(src: string): SearchEntry[] {
  const out: SearchEntry[] = [];
  const re = /section === "([^"]+)"|<Group title="([^"]+)"|<Field\s+label="((?:[^"\\]|\\.)+)"(?:\s+description="((?:[^"\\]|\\.)*)")?|pathRow\("([^"]+)"/g;
  let section = "General";
  let group = "";
  for (let m = re.exec(src); m; m = re.exec(src)) {
    if (m[1]) {
      section = m[1];
      group = "";
      out.push({ title: section, path: ["Settings"], page: "settings", section, kind: "page" });
    } else if (m[2]) {
      group = m[2];
    } else {
      const title = unescape(m[3] ?? m[5]);
      out.push({ title, path: ["Settings", section, ...(group && group !== section ? [group] : [])], page: "settings", section, setting: title, description: m[4] ? unescape(m[4]) : undefined, keywords: SETTING_KEYWORDS[title], kind: "setting" });
    }
  }
  return out;
}

export const INDEX: SearchEntry[] = [
  ...PAGES.map((p) => ({ title: p.title, path: [], page: p.page, keywords: p.keywords, kind: "page" as const })),
  ...FEATURES.map((f) => ({ ...f, kind: "feature" as const })),
  ...settingsEntries(settingsSource),
];

const norm = (t: string) => t.toLowerCase().replace(/[^a-z0-9 ]+/g, " ");

export function search(query: string, index: SearchEntry[] = INDEX, limit = 8): SearchEntry[] {
  const words = norm(query).split(/\s+/).filter(Boolean);
  if (!words.length) return [];
  const scored: { e: SearchEntry; score: number }[] = [];
  for (const e of index) {
    const title = norm(e.title);
    const hay = `${title} ${norm(e.path.join(" "))} ${norm(e.keywords ?? "")} ${norm(e.description ?? "")}`;
    if (!words.every((w) => hay.includes(w))) continue;
    const q = words.join(" ");
    let score = 0;
    if (title === q) score += 100;
    else if (title.startsWith(q)) score += 60;
    else if (title.includes(q)) score += 40;
    score += words.filter((w) => title.split(" ").some((t) => t.startsWith(w))).length * 10;
    score += words.filter((w) => norm(e.keywords ?? "").includes(w)).length * 4;
    score += words.filter((w) => norm(e.path.join(" ")).includes(w)).length * 15;
    if (e.kind === "page") score += 6;
    scored.push({ e, score });
  }
  // de-duplicate identical title+path
  const seen = new Set<string>();
  return scored
    .sort((a, b) => b.score - a.score || a.e.title.length - b.e.title.length)
    .map((x) => x.e)
    .filter((e) => {
      const k = `${e.title}|${e.path.join("/")}`;
      if (seen.has(k)) return false;
      seen.add(k);
      return true;
    })
    .slice(0, limit);
}
