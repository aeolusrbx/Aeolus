import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/index.css";

// Block the browser context menu and reload shortcuts so the window behaves like a desktop app.
if (!import.meta.env.DEV) {
  window.addEventListener("contextmenu", (e) => {
    const t = e.target as HTMLElement;
    if (!t.closest("input, textarea, .selectable")) e.preventDefault();
  });
  window.addEventListener("keydown", (e) => {
    if (e.key === "F5" || (e.ctrlKey && e.key.toLowerCase() === "r")) e.preventDefault();
  });
}

async function bootstrap() {
  // Browser preview with an in-memory backend (development only; stripped from release builds).
  if (import.meta.env.DEV && import.meta.env.VITE_MOCK) {
    const m = await import("../tests/mockBackend");
    m.installMock();
  }
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}

void bootstrap();
