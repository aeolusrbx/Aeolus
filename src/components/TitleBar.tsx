import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, Minus, Square, X } from "lucide-react";
import { api, inTauri } from "../services/api";
import { cx } from "./ui";
import { Logo } from "./Logo";
import { useApp } from "../stores/app";

export function TitleBar({ title }: { title: string }) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    if (!inTauri()) return;
    const w = getCurrentWindow();
    const sync = () => w.isMaximized().then(setMaximized).catch(() => {});
    sync();
    const un = w.onResized(sync);
    return () => {
      un.then((u) => u());
    };
  }, []);

  const { settings } = useApp();
  const iconOnly = settings?.appearance.logoPresentation === "icon";
  const w = () => getCurrentWindow();
  const btn = "flex h-full w-11 items-center justify-center text-fg-2 transition-colors hover:bg-fg/[0.07] hover:text-fg";

  return (
    <div className="flex h-9 shrink-0 select-none items-center bg-bg-2" data-tauri-drag-region onDoubleClick={() => inTauri() && w().toggleMaximize()}>
      <div className="pointer-events-none flex items-center gap-2.5 pl-4" data-tauri-drag-region>
        <Logo size={16} />
        {!iconOnly && <span className="text-xs text-fg-2">Aeolus</span>}
        <span className="sr-only">{title}</span>
      </div>
      <div className="flex-1" data-tauri-drag-region />
      <div className="flex h-full" onDoubleClick={(e) => e.stopPropagation()}>
        <button className={btn} aria-label="Minimize" onClick={() => inTauri() && w().minimize()}>
          <Minus size={15} />
        </button>
        <button className={btn} aria-label={maximized ? "Restore" : "Maximize"} onClick={() => inTauri() && w().toggleMaximize()}>
          {maximized ? <Copy size={12} className="-scale-x-100" /> : <Square size={12} />}
        </button>
        <button className={cx(btn, "hover:!bg-[#c42b1c] hover:!text-white")} aria-label="Close" onClick={() => inTauri() && api.closeWindow()}>
          <X size={16} />
        </button>
      </div>
    </div>
  );
}
