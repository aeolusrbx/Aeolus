import { lazy, Suspense } from "react";
import { AnimatePresence, MotionConfig, motion } from "framer-motion";
import { AppProvider, useApp } from "./stores/app";
import { ToastProvider } from "./stores/toasts";
import { DialogProvider } from "./stores/dialogs";
import { MainLayout } from "./layouts/MainLayout";
import { LoadingScreen } from "./components/LoadingScreen";
import { errorMessage } from "./services/api";
import Home from "./pages/Home";
import type { PageId } from "./types";

const Installations = lazy(() => import("./pages/Installations"));
const Launch = lazy(() => import("./pages/Launch"));
const FastFlags = lazy(() => import("./pages/FastFlags"));
const Customization = lazy(() => import("./pages/Customization"));
const Mods = lazy(() => import("./pages/Mods"));
const Plugins = lazy(() => import("./pages/Plugins"));
const Diagnostics = lazy(() => import("./pages/Diagnostics"));
const SettingsPage = lazy(() => import("./pages/Settings"));

const PAGES: Record<PageId, React.ComponentType> = {
  home: Home,
  installations: Installations,
  launch: Launch,
  fastflags: FastFlags,
  mods: Mods,
  customization: Customization,
  plugins: Plugins,
  diagnostics: Diagnostics,
  settings: SettingsPage,
};

function Shell() {
  const { settings, page, loadError } = useApp();
  if (loadError) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
        <div className="text-[15px] font-semibold text-fg">Aeolus could not start</div>
        <div className="selectable max-w-lg text-[13px] text-fg-2">{errorMessage(loadError)}</div>
      </div>
    );
  }
  if (!settings) return <LoadingScreen />;
  const Page = PAGES[page];
  const anim = settings.appearance.animations;
  return (
    <MotionConfig reducedMotion={anim === "enabled" ? "never" : "always"} transition={anim === "disabled" ? { duration: 0 } : undefined}>
      <MainLayout>
        <AnimatePresence mode="wait">
          <motion.div key={page} className="h-full" initial={{ opacity: 0, y: 4 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} transition={{ duration: 0.14 }}>
            <Suspense fallback={<LoadingScreen inline />}>
              <Page />
            </Suspense>
          </motion.div>
        </AnimatePresence>
      </MainLayout>
    </MotionConfig>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <DialogProvider>
        <AppProvider>
          <Shell />
        </AppProvider>
      </DialogProvider>
    </ToastProvider>
  );
}
