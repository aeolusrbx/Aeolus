import { getCurrentWindow } from "@tauri-apps/api/window";
import { ArrowRight, ChevronRight, Gamepad2, Info, Loader2, MessageCircle, Minus, Settings, X } from "lucide-react";
import { useState, type ReactNode } from "react";
import { api, errorMessage, inTauri, isCancelled } from "../services/api";
import { useApp } from "../stores/app";
import { useToast } from "../stores/toasts";
import { useResource } from "../hooks/useResource";
import { useRobloxActions } from "../hooks/useRobloxActions";
import { gameName } from "./GameHistory";
import { Logo } from "./Logo";
import { ProgressView } from "./ProgressDock";
import { cx } from "./ui";

function MenuButton({ icon, label, sub, onClick, busy, disabled }: { icon: ReactNode; label: string; sub?: string; onClick: () => void; busy?: boolean; disabled?: boolean }) {
  return (
    <button
      onClick={onClick}
      disabled={disabled || busy}
      className="group flex h-[54px] w-full items-center gap-4 rounded-md bg-surface px-4 text-left transition-colors hover:bg-surface-hover disabled:opacity-60"
    >
      <span className="flex w-5 shrink-0 justify-center text-fg">{busy ? <Loader2 size={18} className="animate-spin" /> : icon}</span>
      <span className="min-w-0 flex-1">
        <span className="block truncate text-[14px] text-fg">{label}</span>
        {sub && <span className="block truncate text-[11.5px] text-fg-2">{sub}</span>}
      </span>
      <ChevronRight size={16} className="shrink-0 text-fg-2 transition-transform group-hover:translate-x-0.5" />
    </button>
  );
}

function LinkButton({ icon, children, onClick }: { icon: ReactNode; children: ReactNode; onClick: () => void }) {
  return (
    <button onClick={onClick} className="flex items-center gap-2 rounded px-1.5 py-1 text-[13px] text-fg-2 transition-colors hover:text-fg">
      {icon}
      {children}
    </button>
  );
}

/** The small menu shown when Aeolus opens, like other bootstrappers: launch, or open the full window. */
export function LaunchMenu() {
  const { info, openManager, progress, settings } = useApp();
  const actions = useRobloxActions();
  const toast = useToast();
  const games = useResource(api.gamesOverview, ["games"]);
  const [playing, setPlaying] = useState(false);
  const busy = !!progress && !progress.done;
  const last = games.data?.history[0];

  // Once Roblox is on its way, get out of the way (Aeolus keeps running in the tray).
  const done = () => {
    if (inTauri() && settings?.general.showLaunchMenu) void getCurrentWindow().hide();
  };

  const launch = async () => {
    const r = await actions.launch();
    if (r) done();
  };

  const playLast = async () => {
    if (!last) return;
    setPlaying(true);
    try {
      await api.gameRejoin(last.id, false);
      done();
    } catch (e) {
      if (!isCancelled(e)) toast.error("Couldn't launch", errorMessage(e));
    } finally {
      setPlaying(false);
    }
  };

  const win = "flex h-full w-11 items-center justify-center text-fg-2 transition-colors hover:bg-fg/[0.07] hover:text-fg";

  return (
    <div className="flex h-full flex-col bg-bg">
      <div className="flex h-9 shrink-0 select-none items-center" data-tauri-drag-region>
        <div className="pointer-events-none flex items-center gap-2 pl-3.5" data-tauri-drag-region>
          <Logo size={15} />
          <span className="text-xs text-fg-2">Aeolus</span>
        </div>
        <div className="flex-1" data-tauri-drag-region />
        <div className="flex h-full">
          <button className={win} aria-label="Minimize" onClick={() => inTauri() && getCurrentWindow().minimize()}>
            <Minus size={15} />
          </button>
          <button className={cx(win, "hover:!bg-[#c42b1c] hover:!text-white")} aria-label="Close" onClick={() => inTauri() && api.closeWindow()}>
            <X size={16} />
          </button>
        </div>
      </div>

      <div className="flex min-h-0 flex-1 gap-6 px-6 pb-6 pt-2">
        <div className="flex w-[220px] shrink-0 flex-col">
          <div className="flex flex-1 items-center gap-4">
            <Logo size={64} />
            <div className="min-w-0">
              <div className="text-[26px] font-semibold leading-tight text-fg">Aeolus</div>
              <div className="text-[12.5px] text-fg-2">Version {info?.version ?? "…"}</div>
            </div>
          </div>
          <div className="flex flex-col items-start gap-0.5">
            <LinkButton icon={<Info size={15} />} onClick={() => openManager("settings", { section: "About" })}>About Aeolus</LinkButton>
            <LinkButton icon={<MessageCircle size={15} />} onClick={() => api.openLink("discord").catch(() => undefined)}>Join our Discord server</LinkButton>
          </div>
        </div>

        <div className="flex min-w-0 flex-1 flex-col justify-center gap-2">
          {busy ? (
            <div className="rounded-md bg-surface p-4">
              <ProgressView />
            </div>
          ) : (
            <>
              <MenuButton icon={<ArrowRight size={18} />} label="Launch Roblox" onClick={launch} busy={actions.launching} />
              <MenuButton icon={<Settings size={18} />} label="Configure settings" onClick={() => openManager()} />
              {last && <MenuButton icon={<Gamepad2 size={18} />} label={`Play ${gameName(last)}`} sub="Last played" onClick={playLast} busy={playing} disabled={actions.launching} />}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
