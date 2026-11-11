import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, EVENTS, errorMessage, inTauri } from "../services/api";
import type { AppInfo, PageId, Progress, Settings } from "../types";
import { lighten } from "../utils/format";
import { useToast, type ToastLevel } from "./toasts";
import { useChanged } from "../hooks/useResource";

interface AppCtx {
  settings: Settings | null;
  info: AppInfo | null;
  page: PageId;
  navigate: (p: PageId) => void;
  /** Persist a settings change; reverts on failure. */
  updateSettings: (fn: (s: Settings) => Settings) => Promise<boolean>;
  progress: Progress | null;
  loadError: unknown;
}

const Ctx = createContext<AppCtx | null>(null);

function resolveTheme(t: Settings["general"]["theme"]): "dark" | "light" {
  if (t === "system") return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "light" : "dark";
  return t;
}

export function applyAppearance(s: Settings) {
  const root = document.documentElement;
  root.dataset.theme = resolveTheme(s.general.theme);
  root.style.setProperty("--accent", s.general.accent);
  root.style.setProperty("--accent-2", lighten(s.general.accent, 0.25));
  root.classList.toggle("no-anim", s.appearance.animations === "disabled");
}

export function AppProvider({ children }: { children: ReactNode }) {
  const toast = useToast();
  const [settings, setSettings] = useState<Settings | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [page, setPage] = useState<PageId>("home");
  const [progress, setProgress] = useState<Progress | null>(null);
  const [loadError, setLoadError] = useState<unknown>(null);

  const loadSettings = useCallback(async () => {
    const s = await api.getSettings();
    setSettings(s);
    applyAppearance(s);
    return s;
  }, []);

  useEffect(() => {
    (async () => {
      try {
        const [s, i] = await Promise.all([loadSettings(), api.appInfo()]);
        setInfo(i);
        setPage(s.general.defaultPage);
        for (const n of i.startupNotes) toast.warning("Configuration recovered", n);
      } catch (e) {
        setLoadError(e);
      } finally {
        if (inTauri()) api.appReady().catch(() => {});
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useChanged(["settings"], () => void loadSettings().catch(() => {}));

  // follow OS theme changes when "system" is selected
  useEffect(() => {
    if (!settings || settings.general.theme !== "system") return;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const h = () => applyAppearance(settings);
    mq.addEventListener("change", h);
    return () => mq.removeEventListener("change", h);
  }, [settings]);

  useEffect(() => {
    if (!inTauri()) return;
    const uns: Promise<() => void>[] = [
      listen<Progress>(EVENTS.progress, (e) => {
        setProgress(e.payload);
        if (e.payload.done) window.setTimeout(() => setProgress((p) => (p && p.done ? null : p)), 2500);
      }),
      listen<string>(EVENTS.navigate, (e) => setPage(e.payload as PageId)),
      listen<{ level: ToastLevel; title: string; description: string }>(EVENTS.toast, (e) =>
        toast.push({ level: e.payload.level, title: e.payload.title, description: e.payload.description }),
      ),
    ];
    return () => uns.forEach((p) => p.then((u) => u()));
  }, [toast]);

  const updateSettings = useCallback(
    async (fn: (s: Settings) => Settings) => {
      if (!settings) return false;
      const prev = settings;
      const next = fn(structuredClone(settings));
      setSettings(next);
      applyAppearance(next);
      try {
        const saved = await api.saveSettings(next);
        setSettings(saved);
        applyAppearance(saved);
        return true;
      } catch (e) {
        setSettings(prev);
        applyAppearance(prev);
        toast.error("Setting not saved", errorMessage(e));
        return false;
      }
    },
    [settings, toast],
  );

  const value = useMemo(
    () => ({ settings, info, page, navigate: setPage, updateSettings, progress, loadError }),
    [settings, info, page, updateSettings, progress, loadError],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useApp(): AppCtx {
  const c = useContext(Ctx);
  if (!c) throw new Error("useApp outside AppProvider");
  return c;
}
