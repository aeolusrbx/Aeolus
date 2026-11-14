import type { ReactNode } from "react";
import { TitleBar } from "../components/TitleBar";
import { Sidebar, NAV } from "../components/Sidebar";
import { ProgressDock } from "../components/ProgressDock";
import { useApp } from "../stores/app";
import { useSearchHighlight } from "../components/SearchBox";

export function MainLayout({ children }: { children: ReactNode }) {
  const { page } = useApp();
  const label = NAV.find((n) => n.id === page)?.label;
  useSearchHighlight();
  return (
    <div className="flex h-full flex-col bg-bg-2">
      <TitleBar title={label ? `Aeolus — ${label}` : "Aeolus"} />
      <div className="flex min-h-0 flex-1">
        <Sidebar />
        <main className="min-w-0 flex-1 overflow-y-auto bg-bg">{children}</main>
      </div>
      <ProgressDock />
    </div>
  );
}
