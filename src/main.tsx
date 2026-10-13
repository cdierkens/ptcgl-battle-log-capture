import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { type ReactNode, StrictMode } from "react";
import { createRoot } from "react-dom/client";

import "./styles/globals.css";
import { TooltipProvider } from "./components/ui/tooltip";
import { followSystemColorScheme } from "./lib/color-scheme";
import { MainWindow } from "./MainWindow";
import { CaptureDrawer } from "./overlay/CaptureDrawer";
import { ContinueShield } from "./overlay/ContinueShield";

function rootFor(label: string): ReactNode {
  switch (label) {
    case "capture-drawer":
      return <CaptureDrawer />;
    case "continue-shield":
      return <ContinueShield />;
    default:
      return <MainWindow />;
  }
}

followSystemColorScheme(document.documentElement, window.matchMedia("(prefers-color-scheme: dark)"));

const label = getCurrentWebviewWindow().label;
if (label !== "main") {
  // Overlay windows are transparent; only their content should paint.
  document.documentElement.classList.add("bg-transparent");
  document.body.classList.add("bg-transparent");
}

const rootEl = document.getElementById("root");
if (rootEl === null) {
  throw new Error("#root element not found");
}

createRoot(rootEl).render(
  <StrictMode>
    <TooltipProvider>{rootFor(label)}</TooltipProvider>
  </StrictMode>,
);
