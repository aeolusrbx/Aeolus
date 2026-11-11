import { useState } from "react";
import { api, errorMessage, isCancelled } from "../services/api";
import { useDialogs } from "../stores/dialogs";
import { useToast } from "../stores/toasts";
import { useApp } from "../stores/app";
import { formatBytes } from "../utils/format";
import type { LaunchRecord } from "../types";

export type InstallKind = "install" | "update" | "repair" | "reinstall";

/** Install / update / launch flows with consistent confirmation and error reporting. */
export function useRobloxActions() {
  const dialogs = useDialogs();
  const toast = useToast();
  const { navigate, settings } = useApp();
  const [launching, setLaunching] = useState(false);
  const [checking, setChecking] = useState(false);

  const runInstall = async (kind: InstallKind) => {
    if (kind === "install") {
      const ok = await dialogs.confirm({
        title: "Install Roblox",
        description: (
          <>
            Aeolus will download the current Roblox Player from Roblox's official servers, verify every package and install it into its own folder. Your existing Roblox installation is not changed.
          </>
        ),
        confirmLabel: "Install",
      });
      if (!ok) return;
    }
    if (kind === "reinstall") {
      const ok = await dialogs.confirm({
        title: "Reinstall Roblox?",
        description: "All Roblox files will be downloaded again. The current installation keeps working until the new one is ready. Your settings, profiles and Fast Flags are kept" + (settings?.installation.backupBeforeDestructive ? " and backed up first." : "."),
        confirmLabel: "Reinstall",
        tone: "warning",
      });
      if (!ok) return;
    }
    try {
      const r = await api.runInstall(kind);
      for (const w of r.warnings) toast.warning("Installed with a warning", w);
      return r;
    } catch (e) {
      if (isCancelled(e)) toast.info("Operation cancelled", "Completed downloads were kept and will be reused next time.");
      else dialogs.showError(`Could not ${kind} Roblox`, e);
    }
  };

  const checkUpdates = async (quiet = false) => {
    setChecking(true);
    try {
      const s = await api.checkForUpdates();
      if (!quiet) {
        if (!s.installed) toast.info("Latest Roblox version", s.latest ?? "unknown");
        else if (s.updateAvailable) toast.info("Update available", `${s.installed} → ${s.latest}`);
        else toast.success("Roblox is up to date", s.installed);
      }
      return s;
    } catch (e) {
      if (!quiet) toast.error("Update check failed", errorMessage(e));
    } finally {
      setChecking(false);
    }
  };

  const launch = async (profileId?: string | null): Promise<LaunchRecord | undefined> => {
    setLaunching(true);
    try {
      const r = await api.launchRoblox(profileId);
      toast.success("Roblox launched", r.profile ? `Profile: ${r.profile}` : undefined);
      return r;
    } catch (e) {
      if (isCancelled(e)) return;
      const msg = errorMessage(e);
      if ((e as { kind?: string })?.kind === "not_found" && /not installed/i.test(msg)) {
        const ok = await dialogs.confirm({ title: "Roblox is not installed", description: "Install Roblox now? This downloads it from Roblox's official servers.", confirmLabel: "Install" });
        if (ok) await runInstall("update").then((r) => r && toast.success("Roblox installed", `${r.installed.version} · ${formatBytes(r.installed.size)}`));
        return;
      }
      toast.error("Roblox failed to launch", msg, { label: "Open Diagnostics", onClick: () => navigate("diagnostics") });
    } finally {
      setLaunching(false);
    }
  };

  return { runInstall, checkUpdates, launch, launching, checking };
}
