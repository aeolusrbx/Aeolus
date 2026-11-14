import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { inTauri } from "./api";

export type WindowMode = "launcher" | "manager";

export const LAUNCHER_SIZE = { width: 600, height: 300 };
export const MANAGER_SIZE = { width: 1180, height: 760 };
const MANAGER_MIN = { width: 940, height: 600 };

/** Resize the window for the small launch menu or the full Aeolus window. */
export async function sizeWindow(mode: WindowMode) {
  if (!inTauri()) return;
  const w = getCurrentWindow();
  try {
    if (mode === "launcher") {
      if (await w.isMaximized()) await w.unmaximize();
      await w.setMinSize(new LogicalSize(LAUNCHER_SIZE.width, LAUNCHER_SIZE.height));
      await w.setSize(new LogicalSize(LAUNCHER_SIZE.width, LAUNCHER_SIZE.height));
      await w.setResizable(false);
      await w.setMaximizable(false);
    } else {
      await w.setResizable(true);
      await w.setMaximizable(true);
      await w.setSize(new LogicalSize(MANAGER_SIZE.width, MANAGER_SIZE.height));
      await w.setMinSize(new LogicalSize(MANAGER_MIN.width, MANAGER_MIN.height));
    }
    await w.center();
  } catch {
    /* sizing is cosmetic; never block the app on it */
  }
}
